use crate::prelude::*;

use itertools::Itertools;
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::{EdgeType, Graph, Undirected};

#[derive(Debug, derive_more::Display)]
#[display("")]
pub struct UnitEdge;

#[derive(Debug)]
pub struct Adjacency<ID> {
    graph: Graph<ID, UnitEdge, Undirected>,
    indices: HashMap<ID, NodeIndex<u32>>,
}

impl<ID> Default for Adjacency<ID> {
    fn default() -> Self {
        Self {
            graph: Graph::new_undirected(),
            indices: HashMap::new(),
        }
    }
}

impl<ID: Eq + Hash + Clone + Ord + Display> Adjacency<ID> {
    pub fn add_node(&mut self, peer_id: ID) {
        let index = self.graph.add_node(peer_id.clone());
        self.indices.insert(peer_id, index);
    }

    pub fn add_edge(&mut self, from: ID, to: ID) -> EdgeIndex<u32> {
        self.graph
            .add_edge(self.get_node(from), self.get_node(to), UnitEdge)
    }

    pub fn has_edge(&self, from: ID, to: ID) -> bool {
        let from = self.get_node(from);
        let to = self.get_node(to);
        self.graph.contains_edge(from, to) || self.graph.contains_edge(to, from)
    }

    pub fn get_node(&self, peer_id: ID) -> NodeIndex<u32> {
        self.indices[&peer_id]
    }

    pub fn get_neighbors(&self, peer_id: ID) -> BTreeSet<ID> {
        self.graph
            .neighbors(self.get_node(peer_id))
            .map(|i| self.graph.node_weight(i).unwrap())
            .cloned()
            .collect()
    }

    pub fn has_path_between(&self, from: ID, to: ID) -> bool {
        petgraph::algo::has_path_connecting(
            &self.graph,
            self.get_node(from),
            self.get_node(to),
            None,
        )
    }

    pub fn write_dot(&self, filename: &str, colored: impl IntoIterator<Item = (ID, &'static str)>) {
        write_dot(
            filename,
            &self.graph,
            colored
                .into_iter()
                .map(|(p, color)| (self.get_node(p), color))
                .collect::<HashMap<_, _>>(),
        );
    }

    pub fn graph(&self) -> &Graph<ID, UnitEdge, Undirected> {
        &self.graph
    }
}

/// Write a DOT representation of a graph to a file
pub fn write_dot<N, E, U>(
    filename: &str,
    graph: &Graph<N, E, U>,
    colored: HashMap<NodeIndex<u32>, &'static str>,
) where
    N: Display + PartialEq,
    E: Display,
    U: EdgeType,
{
    let dot = to_dot(graph, colored);
    std::fs::write(filename, dot).unwrap();
}

/// Get a DOT representation of a graph
pub fn to_dot<N, E, U>(
    graph: &Graph<N, E, U>,
    colored: HashMap<NodeIndex<u32>, &'static str>,
) -> String
where
    N: Display + PartialEq,
    E: Display,
    U: EdgeType,
{
    use petgraph::dot::Dot;

    let dot = format!(
        "{}",
        Dot::with_attr_getters(
            &graph,
            &[],
            &|_, _| "fontcolor = \"#777777\" color = \"#777777\" ".to_string(),
            &|_, (n, _)| if let Some(color) = colored.get(&n) {
                format!("fontcolor = \"#cccccc\" color = \"{color}\" ")
            } else {
                "fontcolor = \"#777777\" color = \"#444444\" ".to_string()
            }
        )
    );

    // layout=sfdp
    let extra = "
    layout=fdp
    K=0.4
    sep=\"+20\"
    bgcolor=\"#131313\"
";
    dot.replace("graph {", &format!("graph {{\n    {extra}"))
        .replace("digraph {", &format!("digraph {{\n    {extra}"))
}
