use std::collections::HashMap;

use petgraph::{
    algo::tarjan_scc,
    graph::{DiGraph, EdgeIndex, NodeIndex},
    visit::{EdgeRef, IntoEdgesDirected, IntoNeighborsDirected},
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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

fn record_cycle<N, E>(
    graph: &DiGraph<N, E>,
    stack: &mut Vec<NodeIndex>,
    node: NodeIndex,
    cycles: &mut Vec<Vec<EdgeIndex>>,
) {
    let mut path = vec![];
    let mut pointer = stack.len() - 1;

    path.push(stack[pointer]);

    while path.last().copied() != Some(node) {
        pointer -= 1;

        path.push(stack[pointer]);
    }

    if path.len() == 1 {
        // self loop
        path.push(path[0]);
    }

    let edges = path
        .windows(2)
        .filter_map(|window| graph.find_edge(window[0], window[1]))
        .collect();

    cycles.push(edges);
}

fn process_dfs_tree<N, E>(
    graph: &DiGraph<N, E>,
    stack: &mut Vec<NodeIndex>,
    visited: &mut [CycleState],
    cycles: &mut Vec<Vec<EdgeIndex>>,
) {
    while let Some(&last) = stack.last() {
        if let Some(edge) = graph
            .edges_directed(last, Direction::Outgoing)
            .find(|edge| edge.target() == last)
        {
            cycles.push(vec![edge.id()]);
        }

        // no more outgoing neighbours that have been processed, it is safe to remove it from the
        // stack
        if graph
            .neighbors_directed(last, Direction::Outgoing)
            .all(|node| node == last || visited[node.index()] == CycleState::Processed)
        {
            let index = stack.pop().expect("non-empty").index();
            visited[index] = CycleState::Processed;

            continue;
        }

        for node in graph.neighbors_directed(last, Direction::Outgoing) {
            if node == last {
                continue;
            }

            let index = node.index();

            if visited[index] == CycleState::OnStack {
                record_cycle(graph, stack, node, cycles);
            } else if visited[index] == CycleState::Unvisited {
                stack.push(node);

                visited[index] = CycleState::OnStack;
            }
        }
    }
}

// Adapted from https://www.baeldung.com/cs/detecting-cycles-in-directed-graph
fn find_cycles<N, E>(graph: &DiGraph<N, E>) -> Vec<Vec<EdgeIndex>> {
    let mut visited = vec![CycleState::Unvisited; graph.node_count()];
    let mut cycles = vec![];

    for node in graph.node_indices() {
        let index = node.index();

        if visited[index] == CycleState::Unvisited {
            let mut stack = vec![];
            stack.push(node);

            visited[index] = CycleState::OnStack;
            process_dfs_tree(graph, &mut stack, &mut visited, &mut cycles);
        }
    }

    cycles
}

pub struct DependencyAnalyzer<'a> {
    lookup: Lookup,
    graph: Graph<'a>,
}

impl<'a> DependencyAnalyzer<'a> {
    fn add_link(
        graph: &mut TempGraph<'a>,
        lookup: &mut Lookup,
        source: NodeIndex,
        target: &'a VersionedUrl,
        kind: EdgeKind,
    ) {
        let target = lookup.get(target).copied().map_or_else(
            || {
                let index = graph.add_node(None);
                lookup.insert(target.clone(), index);
                index
            },
            |index| index,
        );

        graph.update_edge(source, target, Edge { kind });
    }

    fn outgoing_entity_type(
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

    fn outgoing_property_value(
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

    fn outgoing_property_type(
        graph: &mut TempGraph<'a>,
        lookup: &mut Lookup,
        index: NodeIndex,
        ty: &'a PropertyType,
    ) {
        for value in ty.one_of() {
            Self::outgoing_property_value(graph, lookup, index, value, None);
        }
    }

    fn outgoing(graph: &mut TempGraph<'a>, lookup: &mut Lookup, index: NodeIndex, ty: &'a AnyType) {
        match ty {
            AnyType::Data(_) => {}
            AnyType::Property(ty) => Self::outgoing_property_type(graph, lookup, index, ty),
            AnyType::Entity(ty) => Self::outgoing_entity_type(graph, lookup, index, ty),
        }
    }

    /// Try to resolve all cycles in a graph by boxing individual nodes
    ///
    /// This is by far the most computationally intensive task.
    fn remove_cycles(graph: &mut Graph) {
        let mut iterations: usize = 1024;

        loop {
            // we need to retain the original edge index, we generate this every time, as otherwise
            // our edge indices would get out of sync
            let plain = graph.filter_map(
                |_, _| Some(()),
                |index, weight| (weight.kind == EdgeKind::Plain).then_some(index),
            );

            let cycles = find_cycles(&plain);

            if cycles.is_empty() {
                break;
            }

            let mut occurrences = vec![0usize; plain.edge_count()];

            for cycle in cycles {
                for edge in cycle {
                    occurrences[edge.index()] += 1;
                }
            }

            let mut edges: Vec<_> = plain
                .edge_indices()
                .filter(|edge| occurrences[edge.index()] > 0)
                .collect();

            if edges.is_empty() {
                // should never happen, but in that case we can already stop, as there is no cycle
                break;
            }

            // sort by occurrences then index to stay stable
            edges.sort_by(|a, b| {
                occurrences[a.index()]
                    .cmp(&occurrences[b.index()])
                    .then(a.cmp(b))
            });

            let chosen = *plain.edge_weight(edges[0]).expect("should exist in graph");
            graph
                .edge_weight_mut(chosen)
                .expect("should exist in graph")
                .kind = EdgeKind::Boxed;

            iterations -= 1;

            assert_ne!(
                iterations, 0,
                "unable to recover, found cycle that couldn't be broken, this should never happen!"
            );
        }
    }

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

        let mut graph = graph.filter_map(
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

        Self::remove_cycles(&mut graph);

        Self { graph, lookup }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn remove_cycles_self_loop() {
        let url = VersionedUrl::from_str("https://example.com/v/1").unwrap();

        let mut graph = Graph::new();
        let index = graph.add_node(Node {
            id: &url,
            kind: NodeKind::DataType,
        });

        let edge = graph.add_edge(index, index, Edge {
            kind: EdgeKind::Plain,
        });

        DependencyAnalyzer::remove_cycles(&mut graph);

        let weight = graph.edge_weight(edge).unwrap();
        assert_eq!(weight.kind, EdgeKind::Boxed);
    }

    #[test]
    fn remove_larger_cycle() {
        let a = VersionedUrl::from_str("https://example.com/v/1").unwrap();
        let b = VersionedUrl::from_str("https://example.com/v/2").unwrap();
        let c = VersionedUrl::from_str("https://example.com/v/3").unwrap();

        let mut graph = Graph::new();
        let idx_a = graph.add_node(Node {
            id: &a,
            kind: NodeKind::DataType,
        });
        let idx_b = graph.add_node(Node {
            id: &a,
            kind: NodeKind::DataType,
        });
        let idx_c = graph.add_node(Node {
            id: &a,
            kind: NodeKind::DataType,
        });

        let ab = graph.add_edge(idx_a, idx_b, Edge {
            kind: EdgeKind::Plain,
        });
        let bc = graph.add_edge(idx_b, idx_c, Edge {
            kind: EdgeKind::Plain,
        });
        let ca = graph.add_edge(idx_c, idx_a, Edge {
            kind: EdgeKind::Plain,
        });

        DependencyAnalyzer::remove_cycles(&mut graph);

        assert_eq!(graph.edge_weight(ab).unwrap().kind, EdgeKind::Boxed);
        assert_eq!(graph.edge_weight(bc).unwrap().kind, EdgeKind::Plain);
        assert_eq!(graph.edge_weight(ca).unwrap().kind, EdgeKind::Plain);
    }

    #[test]
    fn correctly_identify_loop_with_self() {}

    #[test]
    fn overlapping_cycles() {}
}
