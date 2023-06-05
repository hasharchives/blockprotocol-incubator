#![no_std]

mod reachable;

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};

use petgraph::{graph::NodeIndex, visit::NodeFiltered, Graph};
use turbine::entity::{Entity, EntityId, EntityVertex};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityNode {
    id: EntityId,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LinkEdge {
    Left,
    Right,
}

pub struct View {
    graph: Graph<EntityNode, LinkEdge>,

    lookup: BTreeMap<EntityId, NodeIndex>,
    exclude: BTreeSet<NodeIndex>,
}

impl View {
    fn empty() -> Self {
        Self {
            graph: Graph::new(),
            lookup: BTreeMap::new(),
            exclude: BTreeSet::new(),
        }
    }

    fn get_or_create(&mut self, entity: EntityId) -> NodeIndex {
        if let Some(node) = self.lookup.get(&entity) {
            return *node;
        }

        let node = self.graph.add_node(EntityNode { id: entity });

        self.lookup.insert(entity, node);

        node
    }

    fn exclude_complement(&mut self, nodes: BTreeSet<NodeIndex>) {
        let indices: BTreeSet<_> = self.graph.node_indices().collect();

        let complement = &indices - &nodes;
        self.exclude = complement;
    }

    pub fn new(entities: &[Entity]) -> Self {
        let mut this = Self::empty();

        for entity in entities {
            let node = this.get_or_create(entity.metadata.record_id.entity_id);

            if let Some(link_data) = entity.link_data {
                let lhs = this.get_or_create(link_data.left_entity_id);
                let rhs = this.get_or_create(link_data.right_entity_id);

                this.graph.add_edge(lhs, node, LinkEdge::Left);
                this.graph.add_edge(node, rhs, LinkEdge::Right);
            }
        }

        this
    }
}
