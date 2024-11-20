//! Experimental feature for producing "Monte-Carlo state diagrams" from
//! random walks through state machines.
//!
// TODO: more documentation and context

use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
};

use petgraph::graph::DiGraph;
use proptest::prelude::{Arbitrary, BoxedStrategy, Strategy};

use crate::{diagram::to_dot, util::first};

use super::Machine;

const MAX_WALKS: usize = 1000000;

#[derive(Debug, Clone)]
pub struct DiagramConfig {
    pub steps: usize,
    pub walks: usize,
    pub ignore_loopbacks: bool,
}

pub fn print_dot_state_diagram<M>(m: M, config: &DiagramConfig)
where
    M: Machine + Clone + Eq + Debug + Hash,
    M::Action: Arbitrary + Clone + Eq + Debug + Hash + 'static,
{
    println!("{}", to_dot(state_diagram(m, &mut (), config)));
}

pub trait MonteCarloDiagramState<M>
where
    M: Machine,
    M::Action: Arbitrary + 'static,
{
    fn on_action(&mut self, action: &M::Action) {}
    fn on_state(&mut self, state: &M) {}
    fn strategy(&self) -> BoxedStrategy<M::Action> {
        M::Action::arbitrary().boxed()
    }
}

impl<M> MonteCarloDiagramState<M> for ()
where
    M: Machine,
    M::Action: Arbitrary + 'static,
{
}

/// Generate a "Monte Carlo state diagram" of this state machine.
// TODO: stop early if graph is saturated (by random walking over node and edge space first).
// TODO: do more branching from intermediate state, not just from initial state (don't just do "walks", explore many actions at each state.)
pub fn state_diagram<M, S>(m: M, state: &mut S, config: &DiagramConfig) -> DiGraph<M, M::Action>
where
    M: Machine + Clone + Eq + Hash + Debug,
    M::Action: Arbitrary + Clone + Eq + Hash + 'static,
    S: MonteCarloDiagramState<M>,
{
    let mut graph = DiGraph::new();
    let mut node_indices = HashMap::new();
    let mut edges = HashSet::new();

    let initial = m.clone();
    let ix = graph.add_node(initial.clone());
    node_indices.insert(initial, ix);

    let mut terminals = HashSet::new();

    let mut walks = 0;
    let mut total_steps = 0;
    let mut num_errors = 0;
    let mut num_terminations = 0;

    'outer: loop {
        let mut prev = ix;
        let (transitions, errors, num_steps, terminated) = take_a_walk(m.clone(), state, config);
        num_errors += errors.len();
        num_terminations += terminated as usize;
        if !errors.is_empty() {
            tracing::debug!("errors: {:#?}", errors);
            dbg!(&errors);
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

            if !(config.ignore_loopbacks && prev == ix) {
                if edges.insert((prev, ix, edge.clone())) {
                    graph.add_edge(prev, ix, edge);
                }
                prev = ix;
                if terminals.insert(node) {
                    // TODO: can stop here if we know that all terminals have been reached.
                }
            }
            if walks >= config.walks {
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
fn take_a_walk<M, S>(
    mut m: M,
    state: &mut S,
    config: &DiagramConfig,
) -> (Vec<(M::Action, M)>, Vec<M::Error>, usize, bool)
where
    M: Machine + Debug + Clone + Hash + Eq,
    M::Action: Arbitrary + Clone + 'static,
    S: MonteCarloDiagramState<M>,
{
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::default();
    let steps = config.steps;
    let mut transitions = vec![];
    let mut num_steps = 0;
    let mut errors = vec![];
    let mut terminated = false;
    while num_steps < steps {
        num_steps += 1;
        let action: M::Action = state.strategy().new_tree(&mut runner).unwrap().current();

        state.on_action(&action);

        match m.clone().transition(action.clone()).map(first) {
            Ok(mm) => {
                m = mm;
                transitions.push((action, m.clone()));
                state.on_state(&m);
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
