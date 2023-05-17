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

static LINK_REF: Lazy<EntityTypeReference> = Lazy::new(|| {
    EntityTypeReference::new(
        VersionedUrl::from_str(
            "https://blockprotocol.org/@blockprotocol/types/entity-type/link/v/1",
        )
        .expect("should be valid url"),
    )
});

// We cannot handle lifetimes here, because we own the data already, doing so would create a
// self-referential struct, which is considered a war crime in some states.
pub(crate) struct UnificationAnalyzer {
    stack: Vec<VersionedUrl>,
    cache: HashMap<VersionedUrl, AnyType>,
    fetch: Option<Box<dyn FnMut(&VersionedUrl) -> Option<AnyType>>>,
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

        if let Some(any) = self.cache.get(id) {
            return Ok(any);
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
        Ok(&*self.cache.entry(any.id().clone()).or_insert(any))
    }

    pub(crate) fn unify_entity(
        &mut self,
        id: &VersionedUrl,
        entity: &mut EntityType,
    ) -> Result<(), AnalysisError> {
        let mut errors = ErrorAccumulator::new();
        let inherits_from = entity.inherits_from();

        let mut stack = inherits_from.all_of().to_vec();

        while let Some(entry) = stack.pop() {
            let url = entry.url();

            if let Some(ok) = errors.push(self.fetch(url)) {
                match ok {
                    AnyType::Entity(entity) => {
                        if entity.id() == LINK_REF.url() {
                            self.facts.links.insert(id.clone());

                            continue;
                        }

                        let properties = entity.properties().clone();
                        entity.properties();

                        stack.extend(entity.inherits_from().all_of().into_iter().cloned());
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

        let any = &mut self.cache[&id];

        let result = match any {
            AnyType::Entity(entity) => self.unify_entity(&id, entity),
            other => Err(Report::new(AnalysisError::UnsupportedUnification {
                kind: NodeKind::from_any(other),
            })),
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
