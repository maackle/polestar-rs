//! Experimental feature for producing "Monte-Carlo state diagrams" from
//! random walks through state machines.
//!
//! TODO: more documentation and context

use std::collections::{HashMap, HashSet};

use petgraph::graph::DiGraph;
use proptest::prelude::Arbitrary;

use crate::Fsm;

pub fn to_dot<N, E>(graph: DiGraph<N, E>) -> String
where
    N: core::fmt::Debug,
    E: core::fmt::Debug,
{
    use petgraph::dot::Dot;
    format!("{:?}", Dot::with_config(&graph, &[]))
}

/// Generate a "Monte Carlo state diagram" of this state machine.
pub fn state_diagram<M>(m: M, walks: u32, walk_len: u32) -> DiGraph<M, M::Event>
where
    M: Fsm + Clone + Eq + std::hash::Hash,
    M::Event: Arbitrary + Clone + Eq + std::hash::Hash,
{
    let mut graph = DiGraph::new();
    let mut node_indices = HashMap::new();
    let mut edges = HashSet::new();

    let initial = m.clone();
    let ix = graph.add_node(initial.clone());
    node_indices.insert(initial, ix);

    for _ in 0..walks {
        let mut prev = ix;
        for (edge, state) in take_a_walk(m.clone(), walk_len) {
            let node = state.into();
            let ix = if let Some(ix) = node_indices.get(&node) {
                *ix
            } else {
                let ix = graph.add_node(node.clone());
                node_indices.insert(node, ix);
                ix
            };
            if edges.insert((prev, ix, edge.clone())) {
                graph.add_edge(prev, ix, edge);
            }
            prev = ix;
        }
    }

    graph
}

fn take_a_walk<M>(mut m: M, len: u32) -> Vec<(M::Event, M)>
where
    M: Fsm + Clone,
    M::Event: Arbitrary + Clone,
{
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::default();
    let mut steps = vec![];
    for _ in 0..len {
        let event: M::Event = M::Event::arbitrary()
            .new_tree(&mut runner)
            .unwrap()
            .current();
        m.transition(event.clone());
        steps.push((event, m.clone()));
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_derive::{FromPrimitive, ToPrimitive};
    use num_traits::{FromPrimitive, ToPrimitive};
    use proptest::prelude::*;
    use proptest_derive::Arbitrary;

    #[derive(
        Copy, Clone, Debug, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive, derive_more::Display,
    )]
    enum Cycle {
        A,
        B,
        C,
        D,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Arbitrary, ToPrimitive)]
    enum Turn {
        One = 1,
        Two = 2,
    }

    impl Fsm for Cycle {
        type Event = Turn;
        type Fx = ();

        fn transition(&mut self, turn: Turn) {
            let n = turn.to_i8().unwrap();
            *self = Cycle::from_i8((self.to_i8().unwrap() + n).rem_euclid(4)).unwrap()
        }
    }

    #[test]
    fn test_state_diagram() {
        let graph1 = state_diagram(Cycle::D, 10, 10);
        let nodes1: HashSet<_> = graph1.node_weights().collect();
        let edges1: HashSet<_> = graph1.edge_weights().collect();

        let graph2 = state_diagram(Cycle::D, 10, 10);
        let nodes2: HashSet<_> = graph2.node_weights().collect();
        let edges2: HashSet<_> = graph2.edge_weights().collect();

        assert_eq!(nodes1, nodes2);
        assert_eq!(edges1, edges2);
        println!("{}", to_dot(graph1));
    }
}
