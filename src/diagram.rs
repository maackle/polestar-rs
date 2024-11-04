//! Experimental feature for producing "Monte-Carlo state diagrams" from
//! random walks through state machines.
//!
// TODO: Walks don't have to be totally random. We can generate all possible Events and BFS the state tree.
// TODO: more documentation and context

use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use petgraph::graph::DiGraph;
use proptest::prelude::Arbitrary;

use crate::Fsm;

const MAX_WALKS: usize = 1000000;

pub fn print_dot_state_diagram<M>(m: M, stop: impl Into<StopCondition<M>>, min_walks: usize)
where
    M: Fsm + Clone + Eq + std::fmt::Debug + std::hash::Hash,
    M::Event: Arbitrary + Clone + Eq + std::fmt::Debug + std::hash::Hash,
{
    println!("{}", to_dot(state_diagram(m, stop, min_walks)));
}

pub fn to_dot<N, E>(graph: DiGraph<N, E>) -> String
where
    N: core::fmt::Debug,
    E: core::fmt::Debug,
{
    use petgraph::dot::Dot;
    format!("{:?}", Dot::with_config(&graph, &[]))
}

/// Generate a "Monte Carlo state diagram" of this state machine.
// TODO: stop early if graph is saturated (by random walking over node and edge space first).
pub fn state_diagram<M>(
    m: M,
    stop: impl Into<StopCondition<M>>,
    min_walks: usize,
) -> DiGraph<M, M::Event>
where
    M: Fsm + Clone + Eq + std::hash::Hash + std::fmt::Debug,
    M::Event: Arbitrary + Clone + Eq + std::hash::Hash,
{
    let stop = stop.into();

    let mut graph = DiGraph::new();
    let mut node_indices = HashMap::new();
    let mut edges = HashSet::new();

    let initial = m.clone();
    let ix = graph.add_node(initial.clone());
    node_indices.insert(initial, ix);

    let mut terminals = HashSet::new();
    let mut terminals_reached = !matches!(stop, StopCondition::Terminals(_));

    let mut walks = 0;
    let mut total_steps = 0;

    'outer: loop {
        let mut prev = ix;
        let steps = take_a_walk(m.clone(), &stop);
        total_steps += steps.len();
        for (edge, node) in steps {
            let ix = if let Some(ix) = node_indices.get(&node) {
                *ix
            } else {
                let ix = graph.add_node(node.clone());
                node_indices.insert(node.clone(), ix);
                ix
            };
            if edges.insert((prev, ix, edge.clone())) {
                graph.add_edge(prev, ix, edge);
            }
            prev = ix;
            if terminals.insert(node) {
                if let StopCondition::Terminals(ref t) = stop {
                    terminals_reached = terminals.intersection(t).count() == t.len();
                }
            }
            if walks >= MAX_WALKS || terminals_reached && walks >= min_walks {
                break 'outer;
            }
        }
        walks += 1;
    }

    tracing::info!(
        "constructed state diagram in {total_steps} total steps over {walks} walks. nodes={}, edges={}",
        graph.node_count(),
        graph.edge_count()
    );

    graph
}

/// Lets the graph generator know when to stop a random walk
#[derive(Debug, Clone, derive_more::From)]
pub enum StopCondition<M: Eq + Hash> {
    /// Stop after a given number of steps
    Steps { steps: usize },
    /// Stop after reaching any of the given terminals.
    /// Also, walks will continue past the max_walks until all terminals are reached.
    Terminals(HashSet<M>),
}

impl<M: Eq + Hash> From<Vec<M>> for StopCondition<M> {
    fn from(v: Vec<M>) -> Self {
        StopCondition::Terminals(v.into_iter().collect())
    }
}

fn take_a_walk<M>(mut m: M, stop: &StopCondition<M>) -> Vec<(M::Event, M)>
where
    M: Fsm + Clone + Hash + Eq,
    M::Event: Arbitrary + Clone,
{
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::default();
    let mut steps = vec![];
    while match stop {
        StopCondition::Steps { steps: n, .. } => steps.len() < *n,
        StopCondition::Terminals(terminals) => !terminals.contains(&m),
    } {
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
    use proptest_derive::Arbitrary;

    #[derive(
        Copy, Clone, Debug, PartialEq, Eq, FromPrimitive, ToPrimitive, Hash, derive_more::Display,
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
