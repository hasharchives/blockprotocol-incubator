#![no_std]
#![feature(error_in_core)]

mod mutate;
mod reachable;
mod select;

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};

use petgraph::{graph::NodeIndex, Graph};
use turbine::{
    entity::{Entity, EntityId, LinkData},
    VersionedUrl, VersionedUrlRef,
};

const fn no_lookup(_: VersionedUrlRef) -> BTreeSet<VersionedUrlRef<'static>> {
    return BTreeSet::new();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityNode<'a> {
    id: EntityId,

    /// Option<&'a VersionedUrl> is used to allow for incomplete graphs.
    ///
    /// During selection, these are simply ignored.
    type_: Option<&'a VersionedUrl>,
    link_data: Option<&'a LinkData>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LinkEdge {
    Left,
    Right,
}

pub struct View<'a> {
    graph: Graph<EntityNode<'a>, LinkEdge>,

    exclude: BTreeSet<NodeIndex>,

    lookup: BTreeMap<EntityId, NodeIndex>,
    lookup_inherits_from: fn(VersionedUrlRef) -> BTreeSet<VersionedUrlRef<'static>>,
}

impl<'a> View<'a> {
    fn empty() -> Self {
        Self {
            graph: Graph::new(),
            exclude: BTreeSet::new(),

            lookup: BTreeMap::new(),
            lookup_inherits_from: no_lookup,
        }
    }

    fn get_or_create(&mut self, id: EntityId, entity: Option<&'a Entity>) -> NodeIndex {
        if let Some(node) = self.lookup.get(&id) {
            let node = *node;

            if let Some(weight) = self.graph.node_weight_mut(node) {
                if weight.type_.is_none() {
                    if let Some(entity) = entity {
                        weight.type_ = Some(&entity.metadata.entity_type_id);
                        weight.link_data = entity.link_data.as_ref();
                    }
                }
            }

            return node;
        }

        let node = entity.map_or(
            EntityNode {
                id,
                type_: None,
                link_data: None,
            },
            |entity| EntityNode {
                id,
                type_: Some(&entity.metadata.entity_type_id),
                link_data: entity.link_data.as_ref(),
            },
        );

        let node = self.graph.add_node(node);
        self.lookup.insert(id, node);
        node
    }

    fn exclude_complement(&mut self, nodes: &BTreeSet<NodeIndex>) {
        let indices: BTreeSet<_> = self.graph.node_indices().collect();

        let complement = &indices - nodes;
        self.exclude = &complement | &self.exclude;
    }

    fn exclude(&mut self, nodes: &BTreeSet<NodeIndex>) {
        self.exclude = nodes | &self.exclude;
    }

    #[must_use]
    pub fn new(entities: &'a [Entity]) -> Self {
        let mut this = Self::empty();

        for entity in entities {
            let node = this.get_or_create(entity.metadata.record_id.entity_id, Some(entity));

            if let Some(link_data) = entity.link_data {
                let lhs = this.get_or_create(link_data.left_entity_id, None);
                let rhs = this.get_or_create(link_data.right_entity_id, None);

                this.graph.add_edge(lhs, node, LinkEdge::Left);
                this.graph.add_edge(node, rhs, LinkEdge::Right);
            }
        }

        this
    }

    pub fn filter(
        self,
        entities: impl Iterator<Item = &'a Entity>,
    ) -> impl Iterator<Item = &'a Entity> {
        entities.filter(move |entity| {
            let Some(node) = self.lookup.get(&entity.metadata.record_id.entity_id) else {
                return false;
            };

            !self.exclude.contains(node)
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn compile() {}
}
