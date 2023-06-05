use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use core::fmt::{Display, Formatter};
use turbine::entity::EntityId;
use error_stack::{Report, Result};
use petgraph::{Direction, EdgeDirection};
use crate::View;

#[derive(Debug)]
pub enum ReachableError {
    NotFound(EntityId),
}

impl Display for ReachableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "entity not found: {}", id),
        }
    }
}

impl core::error::Error for ReachableError {}

impl View {
    pub fn select_reachable(&mut self, roots: impl Iterator<Item = EntityId>) -> Result<(), ReachableError> {
        let discovered = BTreeSet::new();

        // DFS through the graph
        let mut stack: Vec<_> = roots.collect();

        while let Some(id) = stack.pop() {
            let index = self.lookup.get(&id).copied().ok_or_else(|| Report::new(ReachableError::NotFound(id)))?;

            let mut neighbours: BTreeSet<_> = self.graph.neighbors_directed(index, Direction::Outgoing).collect();

            // remove any nodes that we've already visited or are excluded
            let exclude = &discovered | &self.exclude;
            neighbours = &neighbours - &exclude;

            stack.extend(neighbours);
            neighbours.clear();
        }

        Ok(())
    }

    pub fn select_reachable_undirected(&mut self, roots: impl Iterator<Item = EntityId>) -> Result<(), ReachableError> {
        let discovered = BTreeSet::new();

        // DFS through the graph
        let mut stack: Vec<_> = roots.collect();

        while let Some(id) = stack.pop() {
            let index = self.lookup.get(&id).copied().ok_or_else(|| Report::new(ReachableError::NotFound(id)))?;

            let mut neighbours: BTreeSet<_> = self.graph.neighbors(index).collect();

            // remove any nodes that we've already visited or are excluded
            let exclude = &discovered | &self.exclude;
            neighbours = &neighbours - &exclude;

            stack.extend(neighbours);
            neighbours.clear();
        }

        Ok(())
    }
}
