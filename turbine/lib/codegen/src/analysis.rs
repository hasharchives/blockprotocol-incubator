use std::collections::HashMap;

use ciclo::all_cycles;
use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::IntoNeighborsDirected,
    Direction, EdgeDirection,
};
use type_system::{
    url::VersionedUrl, DataType, EntityType, PropertyType, PropertyTypeReference, PropertyValues,
    ValueOrArray,
};

use crate::{AnyType, AnyTypeRepr};

#[derive(Debug, Copy, Clone)]
pub enum NodeKind {
    DataType,
    PropertyType,
    EntityType,
}

#[derive(Debug, Copy, Clone)]
pub struct Node<'a> {
    id: &'a VersionedUrl,
    kind: NodeKind,
}

#[derive(Debug, Copy, Clone)]
pub enum EdgeKind {
    Plain,
    Boxed,
    Array,
}

#[derive(Debug, Copy, Clone)]
pub struct Edge {
    kind: EdgeKind,
}

type Graph<'a> = DiGraph<Node<'a>, Edge>;
type TempGraph<'a> = DiGraph<Option<Node<'a>>, Edge>;
type Lookup = HashMap<VersionedUrl, NodeIndex>;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum CycleState {
    Unvisited,
    OnStack,
    Processed,
}

fn record_cycle(stack: &mut Vec<NodeIndex>, node: NodeIndex, cycles: &mut Vec<Vec<NodeIndex>>) {
    let mut path = vec![];
    path.push(
        *stack
            .last()
            .expect("stack should have at least one element"),
    );

    while path.last().copied() != Some(node) {
        path.push(*stack.last().expect("stack should be non-empty"));
    }

    cycles.push(path);
}

fn process_dfs_tree(
    graph: &Graph,
    mut stack: &mut Vec<NodeIndex>,
    visited: &mut [CycleState],
    cycles: &mut Vec<Vec<NodeIndex>>,
) {
    while let Some(&last) = stack.last() {
        // no more outgoing neighbours that have been processed, it is safe to remove it from the
        // stack
        if graph
            .neighbors_directed(*last, Direction::Outgoing)
            .all(|index| visited[index] == CycleState::Processed)
        {
            let index = stack.pop().expect("non-empty");
            visited[index] = CycleState::Processed;

            continue;
        }

        for node in graph.neighbors_directed(*last, Direction::Outgoing) {
            if visited[node] == CycleState::OnStack {
                record_cycle(stack, node, cycles);
            } else if visited[node] == CycleState::Unvisited {
                stack.push(node);

                visited[node] = CycleState::OnStack;
            }
        }
    }
}

// Adapted from https://www.baeldung.com/cs/detecting-cycles-in-directed-graph
fn find_cycles(graph: &Graph) -> Vec<Vec<NodeIndex>> {
    let mut visited = vec![CycleState::Unvisited, graph.node_count()];
    let mut cycles = vec![];

    for node in graph.node_indices() {
        if visited[node] == CycleState::Unvisited {
            let mut stack = vec![];
            stack.push(node);

            visited[node] = CycleState::OnStack;
            process_dfs_tree(graph, &mut stack, &mut visited, &mut cycles);
        }
    }

    cycles
}

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
        kind: EdgeKind,
    ) {
        let target = match lookup.get(target).copied() {
            None => {
                let index = graph.add_node(None);
                lookup.insert(target.clone(), index);
                index
            }

            Some(index) => index,
        };

        graph.update_edge(source, target, Edge { kind });
    }

    fn outgoing_entity_type<'a>(
        graph: &mut TempGraph<'a>,
        lookup: &mut Lookup,
        index: NodeIndex,
        ty: &'a EntityType,
    ) {
        let references = ty.properties().values().map(|value| match value {
            ValueOrArray::Value(url) => (url, EdgeKind::Plain),
            ValueOrArray::Array(array) => (array.items(), EdgeKind::Array),
        });

        for (reference, kind) in references {
            Self::add_link(graph, lookup, index, reference.url(), kind);
        }
    }

    fn outgoing_property_value<'a>(
        graph: &mut TempGraph<'a>,
        lookup: &mut Lookup,
        index: NodeIndex,
        value: &'a PropertyValues,
        kind: Option<EdgeKind>,
    ) {
        let kind = kind.unwrap_or(EdgeKind::Plain);

        match value {
            PropertyValues::DataTypeReference(data) => {
                Self::add_link(graph, lookup, index, data.url(), kind)
            }
            PropertyValues::PropertyTypeObject(object) => {
                for value in object.properties().values() {
                    match value {
                        ValueOrArray::Value(value) => {
                            Self::add_link(graph, lookup, index, value.url(), kind)
                        }

                        ValueOrArray::Array(array) => Self::add_link(
                            graph,
                            lookup,
                            index,
                            array.items().url(),
                            EdgeKind::Array,
                        ),
                    }
                }
            }
            PropertyValues::ArrayOfPropertyValues(array) => {
                for value in array.items().one_of() {
                    Self::outgoing_property_value(
                        graph,
                        lookup,
                        index,
                        value,
                        Some(EdgeKind::Array),
                    );
                }
            }
        }
    }

    fn outgoing_property_type<'a>(
        graph: &mut TempGraph<'a>,
        lookup: &mut Lookup,
        index: NodeIndex,
        ty: &'a PropertyType,
    ) {
        for value in ty.one_of() {
            Self::outgoing_property_value(graph, lookup, index, value, None);
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

    /// Try to resolve all cycles in a graph by boxing individual nodes
    ///
    /// This is by far the most computationally intensive task.
    fn cycles(graph: Graph) {}

    pub fn new(types: &'a [AnyType]) -> Self {
        let mut graph = TempGraph::new();
        let mut lookup = Lookup::new();

        for ty in types {
            let node = match ty {
                AnyType::Data(data) => Node {
                    id: data.id(),
                    kind: NodeKind::DataType,
                },
                AnyType::Property(property) => Node {
                    id: property.id(),
                    kind: NodeKind::PropertyType,
                },
                AnyType::Entity(entity) => Node {
                    id: entity.id(),
                    kind: NodeKind::EntityType,
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

            Self::outgoing(&mut graph, &mut lookup, index, ty);
        }

        let graph = graph.filter_map(
            |index, node| {
                if node.is_none() {
                    tracing::warn!(
                        "unable to find definition for type, ignoring in codegen, expect import \
                         errors!"
                    );
                }

                *node
            },
            |index, edge| Some(*edge),
        );

        Self { graph, lookup }
    }
}

// when trying to resolve cycles, use edges that are involved in most cycles, otherwise sort by
// EdgeIndex?
