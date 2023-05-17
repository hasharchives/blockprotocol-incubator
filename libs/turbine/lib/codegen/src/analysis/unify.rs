use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use error_stack::{IntoReport, Report, Result, ResultExt};
use once_cell::sync::Lazy;
use petgraph::{algo::toposort, graph::DiGraph};
use type_system::{repr, url::VersionedUrl, EntityType, EntityTypeReference};

use crate::{
    analysis::{facts::Facts, AnalysisError, NodeKind},
    error::ErrorAccumulator,
    AnyType,
};

pub(super) static LINK_REF: Lazy<EntityTypeReference> = Lazy::new(|| {
    EntityTypeReference::new(
        VersionedUrl::from_str(
            "https://blockprotocol.org/@blockprotocol/types/entity-type/link/v/1",
        )
        .expect("should be valid url"),
    )
});

enum CacheResult<'a> {
    Hit(&'a AnyType),
    Miss(&'a AnyType),
}

type FetchFn = Box<dyn FnMut(&VersionedUrl) -> Option<AnyType>>;

// We cannot handle lifetimes here, because we own the data already, doing so would create a
// self-referential struct, which is considered a war crime in some states.
pub(crate) struct UnificationAnalyzer {
    cache: HashMap<VersionedUrl, AnyType>,
    fetch: Option<FetchFn>,
    facts: Facts,

    missing: HashSet<VersionedUrl>,
}

impl UnificationAnalyzer {
    pub(crate) fn new(values: impl IntoIterator<Item = AnyType>) -> Self {
        let cache: HashMap<_, _> = values
            .into_iter()
            .map(|value| (value.id().clone(), value))
            .collect();

        Self {
            cache,
            fetch: None,
            facts: Facts::new(),

            missing: HashSet::new(),
        }
    }

    pub(crate) fn with_fetch(
        &mut self,
        func: impl FnMut(&VersionedUrl) -> Option<AnyType> + 'static,
    ) {
        self.fetch = Some(Box::new(func));
    }

    pub(crate) fn fetch(&mut self, id: &VersionedUrl) -> Result<CacheResult, AnalysisError> {
        // Optimization, we don't need to query twice, if we know the type is missing
        if self.missing.contains(id) {
            return Err(Report::new(AnalysisError::IncompleteGraph));
        }

        // I'd like to use `.get()` here, but then we get a lifetime error
        if self.cache.contains_key(id) {
            return Ok(CacheResult::Hit(&self.cache[id]));
        }

        let Some(fetch) = &mut self.fetch else {
            self.missing.insert(id.clone());
            return Err(Report::new(AnalysisError::IncompleteGraph))
        };

        let Some(any) = (fetch)(id) else {
            self.missing.insert(id.clone());
            return Err(Report::new(AnalysisError::IncompleteGraph))
        };

        self.cache.insert(any.id().clone(), any);

        Ok(CacheResult::Miss(&self.cache[id]))
    }

    pub(crate) fn entity_or_panic(&mut self, id: &VersionedUrl) -> &EntityType {
        let any = &self.cache[id];

        match any {
            AnyType::Entity(entity) => entity,
            _ => panic!("expected entity"),
        }
    }

    pub(crate) fn remove_entity_or_panic(&mut self, id: &VersionedUrl) -> EntityType {
        let any = self.cache.remove(id).expect("entity not found");

        match any {
            AnyType::Entity(entity) => entity,
            _ => panic!("expected entity"),
        }
    }

    /// This is the main unification function for entity types. It takes an entity type and merges
    /// all parents into it.
    ///
    /// This is done in the following steps:
    ///
    /// A) convert to `repr::EntityType`
    /// B) for every parent in parent:
    ///      1) get the parent
    ///      2) convert to repr::EntityType
    ///      3) merge
    /// C) convert to `Value`
    /// D) set `allOf` again to parents (used later in analysis stage)
    /// E) convert to `repr::EntityType`
    /// F) convert back to `EntityType`
    /// G) insert into cache
    ///
    /// This is only called from `unify`, which already checks for cycles and ensures that every
    /// type exists.
    pub(crate) fn unify_entity(&mut self, id: &VersionedUrl) -> Result<(), AnalysisError> {
        let mut errors = ErrorAccumulator::new();

        let entity = self.remove_entity_or_panic(id);

        let parents: Vec<_> = entity
            .inherits_from()
            .all_of()
            .iter()
            .map(EntityTypeReference::url)
            .cloned()
            .collect();

        let mut entity: repr::EntityType = entity.into();

        for url in parents {
            let parent: repr::EntityType = self.entity_or_panic(&url).clone().into();

            errors.push(entity.merge(parent));
        }

        errors.into_result()?;

        // time to be evil
        let mut entity = serde_json::to_value(entity)
            .into_report()
            .change_context(AnalysisError::UnificationSerde)?;
        entity["allOf"] = parents.into_iter().map(|url| url.to_string()).collect();

        let entity: repr::EntityType = serde_json::from_value(entity)
            .into_report()
            .change_context(AnalysisError::UnificationSerde)?;

        let entity: EntityType = entity
            .try_into()
            .into_report()
            .change_context(AnalysisError::UnificationConvert)?;

        self.cache
            .insert(entity.id().clone(), AnyType::Entity(entity));
        Ok(())
    }

    pub(crate) fn unify(&mut self, id: VersionedUrl) -> Result<(), AnalysisError> {
        let any = &self.cache[&id];

        let result = match any {
            AnyType::Entity(_) => self.unify_entity(&id),
            // currently not supported, so we skip
            _ => Ok(()),
        };

        result
    }

    pub(crate) fn stack(&mut self) -> Result<Vec<VersionedUrl>, AnalysisError> {
        // we will insert things later, therefore we need to clone, not take references
        let mut stack: Vec<_> = self.cache.keys().cloned().collect();

        let mut errors = ErrorAccumulator::new();

        let mut graph = DiGraph::new();
        let mut lookup = HashMap::new();

        while let Some(url) = stack.pop() {
            let entry = &self.cache[&url];

            if let AnyType::Entity(entity) = entry {
                let lhs = *lookup
                    .entry(url.clone())
                    .or_insert_with(|| graph.add_node(url));

                for parent in entity.inherits_from().all_of() {
                    if parent.url() == LINK_REF.url() {
                        continue;
                    }

                    let result = self.fetch(&parent.url());

                    let Some(entry) = errors.push(result) else {
                        continue;
                    };

                    match entry {
                        CacheResult::Miss(any) => stack.push(any.id().clone()),
                        CacheResult::Hit(_) => {}
                    }

                    let rhs = *lookup
                        .entry(parent.url().clone())
                        .or_insert_with(|| graph.add_node(parent.url().clone()));

                    graph.add_edge(lhs, rhs, ());
                }
            }
        }

        errors.into_result()?;

        let mut topo = toposort(&graph, None)
            .map_err(|_error| Report::new(AnalysisError::UnificationCycle))?;

        topo.reverse();

        Ok(topo
            .into_iter()
            .map(|id| graph.node_weight(id).unwrap().clone())
            .collect())
    }

    pub(crate) fn run(mut self) -> Result<(HashMap<VersionedUrl, AnyType>, Facts), AnalysisError> {
        let mut errors = ErrorAccumulator::new();
        let stack = self.stack()?;

        for id in stack {
            errors.push(self.unify(id));
        }

        errors.into_result()?;
        Ok((self.cache, self.facts))
    }
}
