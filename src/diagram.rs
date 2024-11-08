//! Experimental feature for producing "Monte-Carlo state diagrams" from
//! random walks through state machines.
//!
// TODO: Walks don't have to be totally random. We can generate all possible Events and BFS the state tree.
// TODO: more documentation and context

use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
};

use petgraph::graph::DiGraph;
use proptest::prelude::Arbitrary;

use crate::{prelude::*, util::first};

const MAX_WALKS: usize = 1000000;

#[derive(Debug, Clone)]
pub struct DiagramConfig {
    collapse_duplicate_edges: bool,
    ignore_loopbacks: bool,
}

pub fn print_dot_state_diagram<M>(
    m: M,
    config: &DiagramConfig,
    stop: impl Into<StopCondition<M>>,
    min_walks: usize,
) where
    M: Fsm + Clone + Eq + Debug + Hash,
    M::Event: Arbitrary + Clone + Eq + Debug + Hash,
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
    config: &DiagramConfig,
    stop: impl Into<StopCondition<M>>,
    min_walks: usize,
) -> DiGraph<M, M::Event>
where
    M: Fsm + Clone + Eq + Hash + Debug,
    M::Event: Arbitrary + Clone + Eq + Hash,
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
    let mut num_errors = 0;
    let mut num_terminations = 0;

    'outer: loop {
        let mut prev = ix;
        let (transitions, errors, num_steps, terminated) = take_a_walk(m.clone(), &stop);
        num_errors += errors.len();
        num_terminations += terminated as usize;
        if !errors.is_empty() {
            tracing::debug!("errors: {:#?}", errors);
        }
        total_steps += num_steps;
        for (edge, node) in transitions {
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
        "constructed state diagram in {total_steps} total steps ({num_errors} errors, {num_terminations} terminations) over {walks} walks. nodes={}, edges={}",
        graph.node_count(),
        graph.edge_count(),
    );

    graph
}

/// Lets the graph generator know when to stop a random walk
#[derive(Debug, Clone, derive_more::From)]
pub enum StopCondition<M: Eq + Hash> {
    /// Stop after a given number of steps
    Steps { steps: usize },
    /// Stop after reaching any of the given terminals.
    /// Also, walks will continue past the min_walks until all terminals are reached.
    Terminals(HashSet<M>),
}

impl<M: Eq + Hash> From<Vec<M>> for StopCondition<M> {
    fn from(v: Vec<M>) -> Self {
        StopCondition::Terminals(v.into_iter().collect())
    }
}

#[allow(clippy::type_complexity)]
fn take_a_walk<M>(
    mut m: M,
    stop: &StopCondition<M>,
) -> (Vec<(M::Event, M)>, Vec<M::Error>, usize, bool)
where
    M: Fsm + Debug + Clone + Hash + Eq,
    M::Event: Arbitrary + Clone,
{
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::default();
    let mut transitions = vec![];
    let mut num_steps = 0;
    let mut errors = vec![];
    let mut terminated = false;
    while match stop {
        StopCondition::Steps { steps: n, .. } => num_steps < *n,
        StopCondition::Terminals(terminals) => !terminals.contains(&m),
    } {
        num_steps += 1;
        let event: M::Event = M::Event::arbitrary()
            .new_tree(&mut runner)
            .unwrap()
            .current();

        match m.clone().transition(event.clone()).map(first) {
            Ok(mm) => {
                m = mm;
                transitions.push((event, m.clone()));
                if m.is_terminal() {
                    terminated = true;
                    break;
                }
            }
            Err(err) => {
                // TODO: would be better to exhaustively try each event in turn in the error case, so that if all events lead to error, we can halt early.
                errors.push(err);
            }
        };
    }
    (transitions, errors, num_steps, terminated)
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
        type Error = Infallible;

        fn transition(mut self, turn: Turn) -> Result<(Self, Self::Fx), Self::Error> {
            let n = turn.to_i8().unwrap();
            self = Cycle::from_i8((self.to_i8().unwrap() + n).rem_euclid(4)).unwrap();
            Ok((self, ()))
        }

        fn is_terminal(&self) -> bool {
            false
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
