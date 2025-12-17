use crate::prelude::*;

use crate::network::adjacency::Adjacency;

pub enum Topology<ID> {
    FullyConnected,
    Adjacency(Adjacency<ID>),
}

impl<ID: Eq + Hash + Clone + Ord + Display> Topology<ID> {
    pub fn has_edge(&self, a: ID, b: ID) -> bool {
        match self {
            Topology::FullyConnected => true,
            Topology::Adjacency(adj) => adj.has_edge(a, b),
        }
    }

    pub fn has_path_between(&self, a: ID, b: ID) -> bool {
        match self {
            Topology::FullyConnected => true,
            Topology::Adjacency(adj) => adj.has_path_between(a, b),
        }
    }
}
