use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use error_stack::{Report, Result};
use once_cell::sync::Lazy;
use type_system::{url::VersionedUrl, EntityType, EntityTypeReference};

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

type FetchFn = Box<dyn FnMut(&VersionedUrl) -> Option<AnyType>>;

// We cannot handle lifetimes here, because we own the data already, doing so would create a
// self-referential struct, which is considered a war crime in some states.
pub(crate) struct UnificationAnalyzer {
    stack: Vec<VersionedUrl>,
    cache: HashMap<VersionedUrl, AnyType>,
    fetch: Option<FetchFn>,
    facts: Facts,

    visited: HashSet<VersionedUrl>,
    missing: HashSet<VersionedUrl>,
}

impl UnificationAnalyzer {
    pub(crate) fn new(values: impl IntoIterator<Item = AnyType>) -> Self {
        let cache: HashMap<_, _> = values
            .into_iter()
            .map(|value| (value.id().clone(), value))
            .collect();

        let stack = cache.keys().cloned().collect();

        Self {
            cache,
            stack,
            fetch: None,
            facts: Facts::new(),

            visited: HashSet::new(),
            missing: HashSet::new(),
        }
    }

    pub(crate) fn with_fetch(
        &mut self,
        func: impl FnMut(&VersionedUrl) -> Option<AnyType> + 'static,
    ) {
        self.fetch = Some(Box::new(func));
    }

    pub(crate) fn fetch(&mut self, id: &VersionedUrl) -> Result<&AnyType, AnalysisError> {
        if self.missing.contains(id) {
            return Err(Report::new(AnalysisError::IncompleteGraph));
        }

        // I'd like to use `.get()` here, but then we get a lifetime error
        if self.cache.contains_key(id) {
            return Ok(&self.cache[id]);
        }

        let Some(fetch) = &mut self.fetch else {
            self.missing.insert(id.clone());
            return Err(Report::new(AnalysisError::IncompleteGraph))
        };

        let Some(any) = (fetch)(id) else {
            self.missing.insert(id.clone());
            return Err(Report::new(AnalysisError::IncompleteGraph))
        };

        self.stack.push(any.id().clone());
        self.cache.insert(any.id().clone(), any);

        Ok(&self.cache[id])
    }

    pub(crate) fn entity_or_panic(&self, id: &VersionedUrl) -> &EntityType {
        let any = &self.cache[id];

        match any {
            AnyType::Entity(entity) => entity,
            _ => panic!("expected entity"),
        }
    }

    pub(crate) fn unify_entity(&mut self, id: &VersionedUrl) -> Result<(), AnalysisError> {
        let mut errors = ErrorAccumulator::new();

        let inherits_from = self.entity_or_panic(id).inherits_from();
        let mut stack = inherits_from.all_of().to_vec();

        while let Some(entry) = stack.pop() {
            let url = entry.url();

            if url == LINK_REF.url() {
                self.facts.links.insert(id.clone());

                continue;
            }

            if let Some(ok) = errors.push(self.fetch(url)) {
                match ok {
                    AnyType::Entity(entity) => {
                        let properties = entity.properties().clone();
                        entity.properties();

                        stack.extend(entity.inherits_from().all_of().iter().cloned());
                    }
                    other => errors.extend_one(Report::new(AnalysisError::ExpectedNodeKind {
                        expected: NodeKind::EntityType,
                        received: NodeKind::from_any(other),
                        url: other.id().clone(),
                    })),
                }
            }
        }

        errors.into_result()
    }

    pub(crate) fn unify(&mut self, id: VersionedUrl) -> Result<(), AnalysisError> {
        if self.visited.contains(&id) {
            return Ok(());
        }

        // we already errored out once, don't need to do it all over again
        if self.missing.contains(&id) {
            return Ok(());
        }

        let any = &self.cache[&id];

        let result = match any {
            AnyType::Entity(_) => self.unify_entity(&id),
            // currently not supported, so we skip
            _ => Ok(()),
        };

        self.visited.insert(id);
        result
    }

    pub(crate) fn run(mut self) -> Result<(HashMap<VersionedUrl, AnyType>, Facts), AnalysisError> {
        let mut errors = ErrorAccumulator::new();

        while let Some(id) = self.stack.pop() {
            errors.push(self.unify(id));
        }

        errors.into_result()?;
        Ok((self.cache, self.facts))
    }
}
