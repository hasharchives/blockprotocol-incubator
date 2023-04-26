use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};
use type_system::{url::VersionedUrl, DataType, EntityType, PropertyType};

use crate::{AnyType, AnyTypeRepr};

#[derive(Debug, Copy, Clone)]
pub enum NodeType {
    DataType,
    PropertyType,
    EntityType,
}

#[derive(Debug, Copy, Clone)]
pub struct Node<'a> {
    id: &'a VersionedUrl,
    ty: NodeType,
}

#[derive(Debug, Copy, Clone)]
pub struct Edge {}

type Graph<'a> = DiGraph<Node<'a>, Edge>;
type TempGraph<'a> = DiGraph<Option<Node<'a>>, Edge>;
type Lookup = HashMap<VersionedUrl, NodeIndex>;

pub struct Analysis<'a> {
    lookup: Lookup,
    graph: Graph<'a>,
}

impl<'a> Analysis<'a> {
    fn add_link<'a>(
        graph: &mut TempGraph<'a>,
        lookup: &mut Lookup,
        source: NodeIndex,
        target: &VersionedUrl,
    ) {
        let target = match lookup.get(target).copied() {
            None => {
                let index = graph.add_node(None);
                lookup.insert(target.clone(), index);
                index
            }

            Some(index) => index,
        };

        graph.update_edge(source, target, Edge {});
    }

    fn outgoing_entity_type<'a>(
        graph: &mut TempGraph<'a>,
        lookup: &mut Lookup,
        index: NodeIndex,
        ty: &'a EntityType,
    ) {
        for reference in ty.property_type_references() {
            Self::add_link(graph, lookup, index, reference.url());
        }
    }

    fn outgoing_property_type<'a>(
        graph: &mut TempGraph<'a>,
        lookup: &mut Lookup,
        index: NodeIndex,
        ty: &'a PropertyType,
    ) {
        for reference in ty.property_type_references() {
            Self::add_link(graph, lookup, index, reference.url());
        }

        for reference in ty.data_type_references() {
            Self::add_link(graph, lookup, index, reference.url());
        }
    }

    fn outgoing<'a>(
        graph: &mut TempGraph<'a>,
        lookup: &mut Lookup,
        index: NodeIndex,
        ty: &'a AnyType,
    ) {
        match ty {
            AnyType::Data(_) => {}
            AnyType::Property(ty) => Self::outgoing_property_type(graph, lookup, index, ty),
            AnyType::Entity(ty) => Self::outgoing_entity_type(graph, lookup, index, ty),
        }
    }

    pub fn new(types: &'a [AnyType]) {
        let mut graph = TempGraph::new();
        let mut lookup = Lookup::new();

        for ty in types {
            let node = match ty {
                AnyType::Data(data) => Node {
                    id: data.id(),
                    ty: NodeType::DataType,
                },
                AnyType::Property(property) => Node {
                    id: property.id(),
                    ty: NodeType::PropertyType,
                },
                AnyType::Entity(entity) => Node {
                    id: entity.id(),
                    ty: NodeType::EntityType,
                },
            };

            let index = if let Some(index) = lookup.get(ty.id()) {
                let weight = graph
                    .node_weight_mut(*index)
                    .expect("lookup table contract violated");
                *weight = Some(node);

                *index
            } else {
                let index = graph.add_node(Some(node));
                lookup.insert(ty.id().clone(), index);

                index
            };

            // TODO: analyse all the outgoing connections
            Self::outgoing(&mut graph, &mut lookup, index, ty);
        }
    }
}
