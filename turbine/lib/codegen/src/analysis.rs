use petgraph::graph::DiGraph;

#[derive(Debug, Copy, Clone)]
pub struct Node {}

#[derive(Debug, Copy, Clone)]
pub struct Edge {}

pub struct Analysis {
    graph: DiGraph<Node, Edge>,
}
