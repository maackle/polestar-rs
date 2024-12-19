use colored::Colorize;
use exhaustive::Exhaustive;
use human_repr::HumanCount;
use parking_lot::Mutex;
use petgraph::graph::{DiGraph, NodeIndex};

use std::sync::atomic::Ordering::SeqCst;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc,
    },
};

use crate::{util::first, Machine};

#[derive(derive_bounded::Clone, bon::Builder)]
#[bounded_to(M::State)]
#[allow(clippy::type_complexity)]
pub struct TraversalConfig<M: Machine> {
    pub max_actions: Option<usize>,
    pub max_depth: Option<usize>,
    pub max_iters: Option<usize>,
    pub trace_every: Option<usize>,

    pub graphing: Option<TraversalGraphingConfig>,

    #[builder(default)]
    pub ignore_loopbacks: bool,
    #[builder(default)]
    pub record_terminals: bool,
    #[builder(default)]
    pub trace_error: bool,

    #[builder(with = |f: impl Fn(&M::State, VisitType) -> Result<(), M::Error> + Send + Sync + 'static| Arc::new(f))]
    pub visitor: Option<Arc<dyn Fn(&M::State, VisitType) -> Result<(), M::Error> + Send + Sync>>,

    #[builder(with = |f: impl Fn(&M::Error) -> bool + Send + Sync + 'static| Arc::new(f))]
    pub is_fatal_error: Option<Arc<dyn Fn(&M::Error) -> bool + Send + Sync>>,
}

#[derive(Default, derive_bounded::Clone)]
pub struct TraversalGraphingConfig {
    pub ignore_loopbacks: bool,
}

// impl<M: Machine, N, E> TraversalGraphingConfig<M, N, E> {
//     pub fn new(
//         map_node: impl Fn(&M::State) -> N + Send + Sync + 'static,
//         map_edge: impl Fn(&M::Action) -> E + Send + Sync + 'static,
//     ) -> Self {
//         Self {
//             map_node: Arc::new(map_node),
//             map_edge: Arc::new(map_edge),
//         }
//     }
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisitType {
    Normal,
    Terminal,
    LoopTerminal,
}

impl<M: Machine> Default for TraversalConfig<M> {
    fn default() -> Self {
        Self {
            max_actions: None,
            max_depth: None,
            max_iters: None,
            trace_every: None,
            ignore_loopbacks: false,
            record_terminals: false,
            trace_error: false,
            visitor: None,
            is_fatal_error: None,
            graphing: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct TraversalReport {
    pub num_visited: usize,
    pub num_terminations: usize,
    pub num_edges_skipped: usize,
    pub total_steps: usize,
    pub max_depth: usize,
    pub time_taken: std::time::Duration,
}

pub fn traverse<M, S>(
    machine: Arc<M>,
    initial: M::State,
    config: TraversalConfig<M>,
    map_state: impl Fn(M::State) -> Option<S> + Send + Sync + 'static,
) -> Result<
    (
        TraversalReport,
        Option<DiGraph<M::State, M::Action>>,
        Option<(TerminalSet<M::State>, TerminalSet<M::State>)>,
    ),
    M::Error,
>
where
    M: Machine + Send + Sync + 'static,
    M::State: Clone + Eq + Hash + Debug + Send + Sync + 'static,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug + Send + Sync + 'static,
    M::Error: Debug + Send + Sync + 'static,
    S: Clone + Eq + Hash + Debug + Send + Sync + 'static,
{
    let terminals: Arc<Mutex<TerminalSet<M::State>>> = Arc::new(Mutex::new(HashSet::new()));
    let loop_terminals: Arc<Mutex<TerminalSet<M::State>>> = Arc::new(Mutex::new(HashSet::new()));
    let visited_states: Arc<Mutex<HashMap<S, NodeIndex>>> = Arc::new(Mutex::new(HashMap::new()));
    let visited_edges: Arc<Mutex<HashSet<(NodeIndex, NodeIndex, M::Action)>>> =
        Arc::new(Mutex::new(HashSet::new()));

    let graph = Arc::new(Mutex::new(DiGraph::new()));

    let seen = Arc::new(crossbeam::queue::SegQueue::new());

    let (err_tx, err_rx) = crossbeam::channel::bounded(1);

    seen.push((initial, Option::<(NodeIndex, M::Action)>::None, 0));
    // to_visit.lock().push_back((initial, 0));

    let stop = Arc::new(AtomicBool::new(false));
    let total_steps = Arc::new(AtomicUsize::new(0));
    let num_seen = Arc::new(AtomicUsize::new(0));
    let num_terminations = Arc::new(AtomicUsize::new(0));
    let num_edges_skipped = Arc::new(AtomicUsize::new(0));
    let max_depth = Arc::new(AtomicUsize::new(0));

    let all_actions: im::Vector<_> = M::Action::iter_exhaustive(config.max_actions).collect();

    let start_time = std::time::Instant::now();

    let task = {
        let config = config.clone();
        let graph = graph.clone();
        let stop = stop.clone();
        let total_steps = total_steps.clone();
        let num_seen = num_seen.clone();
        let num_terminations = num_terminations.clone();
        let num_edges_skipped = num_edges_skipped.clone();
        let max_depth = max_depth.clone();
        let visited = visited_states.clone();
        let terminals = terminals.clone();
        let loop_terminals = loop_terminals.clone();

        let active_threads = AtomicUsize::new(0);
        let prev_trace = Mutex::new(IterTrace::default());
        let trace_every = config.trace_every.unwrap_or(usize::MAX);

        move |thread_index: usize| {
            if thread_index > 0 {
                // Give the seen queue time to fill up
                // XXX: this is a hack to avoid starvation, could be more robust
                while seen.len() < 1000 {
                    if stop.load(SeqCst) {
                        // The first thread already completed all work
                        return Ok(());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }

            active_threads.fetch_add(1, SeqCst);

            while let Some((state, prev_node, depth)) = seen.pop() {
                if stop.load(SeqCst) {
                    break;
                }
                let iter = total_steps.fetch_add(1, SeqCst);

                if iter > 0 && iter % trace_every == 0 {
                    let trace = IterTrace {
                        iter,
                        queued: num_seen.load(SeqCst),
                        visited: visited.lock().len(),
                        depth,
                    };
                    let mut prev = prev_trace.lock();

                    let queued_diff = trace.queued as isize - prev.queued as isize;
                    let visited_diff = trace.visited as isize - prev.visited as isize;

                    let queued_diff_str = if queued_diff == 0 {
                        "0".to_string().white()
                    } else if queued_diff > 0 {
                        format!("+{}", queued_diff.human_count_bare()).green()
                    } else {
                        format!("{}", queued_diff.human_count_bare()).red()
                    };

                    let visited_diff_str = if visited_diff == 0 {
                        "0".to_string().white()
                    } else if visited_diff > 0 {
                        format!("+{}", visited_diff.human_count_bare()).green()
                    } else {
                        format!("{}", visited_diff.human_count_bare()).red()
                    };

                    let depth_str = if trace.depth != prev.depth {
                        format!("depth={depth} ***").underline().bold()
                    } else {
                        format!("depth={depth}    ").white()
                    };

                    tracing::info!(
                        "iter={:<5} | visited={:<8} Δ={:<8} | queued={:<8} Δ={:<8} | {}",
                        trace.iter.human_count_bare().to_string(),
                        trace.visited.human_count_bare().to_string(),
                        visited_diff_str,
                        trace.queued.human_count_bare().to_string(),
                        queued_diff_str,
                        depth_str,
                    );
                    *prev = trace;
                }

                if config.max_iters.map(|m| iter >= m).unwrap_or(false) {
                    panic!("max iters of {} reached", config.max_iters.unwrap());
                }

                let (already_seen, node_ix) = if let Some(mapped) = map_state(state.clone()) {
                    let mut visited = visited.lock();
                    match visited.entry(mapped) {
                        std::collections::hash_map::Entry::Occupied(entry) => (true, *entry.get()),
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            max_depth.fetch_max(depth, SeqCst);

                            if config.graphing.is_some() {
                                let node_ix = graph.lock().add_node(state.clone());
                                entry.insert(node_ix);
                                (false, node_ix)
                            } else {
                                entry.insert(NodeIndex::end());
                                (false, NodeIndex::end())
                            }
                        }
                    }
                } else {
                    // skip a node with no mapping
                    continue;
                };

                if let Some(g) = config.graphing.as_ref() {
                    let mut graph = graph.lock();
                    if let Some((prev_node_ix, edge)) = prev_node {
                        let ignore = g.ignore_loopbacks && prev_node_ix == node_ix;
                        if !ignore
                            && visited_edges
                                .lock()
                                .insert((prev_node_ix, node_ix, edge.clone()))
                        {
                            let _ = graph.add_edge(prev_node_ix, node_ix, edge);
                        }
                    }
                }

                // Don't explore the same node twice
                if already_seen {
                    if let Some(ref on_terminal) = config.visitor {
                        on_terminal(&state, VisitType::LoopTerminal)?
                    }
                    if config.record_terminals {
                        loop_terminals.lock().insert(state);
                    }

                    continue;
                }

                // If this is a terminal state, no need to explore further.
                if machine.is_terminal(&state) {
                    num_terminations.fetch_add(1, SeqCst);
                    if let Some(ref on_terminal) = config.visitor {
                        on_terminal(&state, VisitType::Terminal)?
                    }
                    if config.record_terminals {
                        terminals.lock().insert(state);
                    }
                    continue;
                } else {
                    if let Some(ref on_terminal) = config.visitor {
                        on_terminal(&state, VisitType::Normal)?
                    }
                }

                // Respect the depth limit
                if depth >= config.max_depth.unwrap_or(usize::MAX) {
                    continue;
                }

                // Queue up visits to all nodes reachable from this node..
                for action in all_actions.iter().cloned() {
                    let prev_node = if config.graphing.is_some() {
                        Some((node_ix, action.clone()))
                    } else {
                        None
                    };
                    match machine.transition(state.clone(), action.clone()).map(first) {
                        Ok(node) => {
                            num_seen.fetch_add(1, SeqCst);
                            seen.push((node, prev_node, depth + 1));
                        }
                        Err(err) => {
                            num_edges_skipped.fetch_add(1, SeqCst);

                            if let Some(ref is_fatal_error) = config.is_fatal_error {
                                if is_fatal_error(&err) {
                                    return Err(err);
                                }
                            } else if config.trace_error {
                                tracing::error!("edge skipped: {:?}", err);
                            }
                        }
                    }
                }
            }

            tracing::trace!("traversal thread {} done", thread_index);
            let current_active_threads = active_threads.fetch_sub(1, SeqCst);

            // handle the case where the first thread rips through all work
            // before any other threads even get started
            if thread_index == 0 && current_active_threads == 1 {
                stop.store(true, SeqCst);
            }
            Ok(())
        }
    };

    const THREADED: bool = true;

    if !THREADED {
        task(0)?;
    } else {
        rayon::spawn_broadcast(move |broadcast_ctx: rayon::BroadcastContext<'_>| {
            if let Err(err) = task(broadcast_ctx.index()) {
                stop.store(true, SeqCst);
                err_tx.send(err).unwrap();
            }
        });

        match err_rx.recv() {
            Ok(err) => {
                return Err(err);
            }
            Err(crossbeam::channel::RecvError) => {
                // success
            }
        }
    }

    let report = TraversalReport {
        time_taken: std::time::Instant::now().duration_since(start_time),
        num_visited: visited_states.lock().len(),
        num_terminations: num_terminations.load(SeqCst),
        num_edges_skipped: num_edges_skipped.load(SeqCst),
        total_steps: total_steps.load(SeqCst),
        max_depth: max_depth.load(SeqCst),
    };
    let terminals = config.record_terminals.then(|| {
        (
            Arc::into_inner(terminals).unwrap().into_inner(),
            Arc::into_inner(loop_terminals).unwrap().into_inner(),
        )
    });
    let graph = config
        .graphing
        .is_some()
        .then_some(Arc::into_inner(graph).unwrap().into_inner());
    Ok((report, graph, terminals))
}

pub type TerminalSet<S> = HashSet<S>;

#[derive(Default)]
pub struct IterTrace {
    pub iter: usize,
    pub queued: usize,
    pub visited: usize,
    pub depth: usize,
}

#[cfg(test)]
mod tests {
    use crate::TransitionResult;

    use super::*;

    #[test]
    #[cfg(todo)]
    fn test_checked_traversal() {
        #[derive(Clone, Debug)]
        struct SimpleMachine;

        const TERMINAL: u16 = 1000;

        #[derive(Clone, Debug, Exhaustive, PartialEq, Eq, Hash)]
        enum Action {
            Double,
            Triple,
        }

        impl Machine for SimpleMachine {
            type State = u16;
            type Action = Action;
            type Fx = ();
            type Error = anyhow::Error;

            fn transition(
                &self,
                state: Self::State,
                action: Self::Action,
            ) -> TransitionResult<Self> {
                match action {
                    Action::Double => Ok((state.saturating_mul(2), ())),
                    Action::Triple => Ok((state.saturating_mul(3), ())),
                }
            }

            fn is_terminal(&self, s: &Self::State) -> bool {
                *s > TERMINAL
            }
        }

        let divby = |n| P::atom(format!("div-by-{}", n), move |s| s % n == 0);
        let big = P::atom("big".to_string(), |s| *s > TERMINAL);
        let checker = SimpleMachine.checked().with_predicates([
            P::always(big.clone().implies(P::next(P::not(big)))),
            P::always(divby(2).or(divby(3))),
            P::eventually(divby(3)),
        ]);

        let err = checker
            .check_fold(1, [Action::Double, Action::Double])
            .unwrap_err();

        assert!(err.unwrap_predicate().error.contains("div-by-3"));

        let initial = checker.initial(1);
        let err = traverse_checked(
            checker,
            initial,
            TraversalConfig {
                ..Default::default()
            },
            Some,
        )
        .unwrap_err();

        assert_eq!(
            err.path,
            std::iter::repeat(Action::Double)
                .take(10)
                .collect::<im::Vector<_>>()
        );
    }
}
