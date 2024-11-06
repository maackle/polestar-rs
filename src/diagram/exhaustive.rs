use exhaustive::Exhaustive;
use petgraph::graph::DiGraph;

use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
};

use crate::Fsm;

/// Generate a "Monte Carlo state diagram" of this state machine.
// TODO: stop early if graph is saturated (by random walking over node and edge space first).
pub fn state_diagram<M>(m: M) -> DiGraph<M, M::Event>
where
    M: Fsm + Clone + Eq + Hash + Debug,
    M::Event: Exhaustive + Clone + Eq + Hash,
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

#[allow(clippy::type_complexity)]
fn take_a_walk<M>(
    mut m: M,
    stop: &StopCondition<M>,
) -> (Vec<(M::Event, M)>, Vec<M::Error>, usize, bool)
where
    M: Fsm + Debug + Clone + Hash + Eq,
    M::Event: Exhaustive + Clone,
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
            }
            Err(err) => {
                if err.is_terminal() {
                    terminated = true;
                    break;
                } else {
                    // TODO: would be better to exhaustively try each event in turn in the error case, so that if all events lead to error, we can halt early.
                    errors.push(err);
                }
            }
        };
    }
    (transitions, errors, num_steps, terminated)
}
