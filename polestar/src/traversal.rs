use colored::Colorize;
use exhaustive::Exhaustive;
use human_repr::HumanCount;
use itertools::Itertools;
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

use crate::logic::{PropMapping, Propositions, Transition};
use crate::machine::Cog;
use crate::model_checker::{ModelCheckerError, ModelCheckerState, ModelCheckerTransitionError};
use crate::prelude::{model_checker_report, ModelChecker};
use crate::{util::first, Machine};

#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct Traversal<M: Machine, S = <M as Machine>::State, A = <M as Machine>::Action> {
    pub machine: M,
    pub initial: im::Vector<M::State>,

    max_depth: Option<usize>,
    trace_every: Option<usize>,
    trace_errors: bool,
    ignore_loopbacks: bool,

    visitor: Arc<dyn Fn(&M::State, VisitType) -> Result<(), M::Error> + Send + Sync>,
    is_fatal_error: Arc<dyn Fn(&M::Error) -> bool + Send + Sync>,
    map_state: Arc<dyn Fn(M::State) -> Option<S> + Send + Sync>,
    map_action: Arc<dyn Fn(M::Action) -> Option<A> + Send + Sync>,
}

impl<M: Machine> Traversal<M>
where
    M::State: Clone + Debug,
    M::Action: Clone + Debug + Exhaustive,
{
    pub fn new(machine: M, initial: impl IntoIterator<Item = M::State>) -> Self {
        Self {
            machine,
            initial: initial.into_iter().collect(),
            max_depth: None,
            trace_every: None,
            trace_errors: false,
            ignore_loopbacks: false,
            visitor: Arc::new(|_, _| Ok(())),
            is_fatal_error: Arc::new(|_| false),
            map_state: Arc::new(|s| Some(s)),
            map_action: Arc::new(|a| Some(a)),
        }
    }
}

impl<M, S, A> Traversal<M, S, A>
where
    M: Machine,
    M::Action: Exhaustive,
    S: Cog + 'static,
    A: Cog + 'static,
{
    pub fn max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = Some(max_depth);
        self
    }

    pub fn trace_every(mut self, trace_every: usize) -> Self {
        self.trace_every = Some(trace_every);
        self
    }

    pub fn ignore_loopbacks(mut self, ignore_loopbacks: bool) -> Self {
        self.ignore_loopbacks = ignore_loopbacks;
        self
    }

    pub fn visitor(
        mut self,
        visitor: impl Fn(&M::State, VisitType) -> Result<(), M::Error> + Send + Sync + 'static,
    ) -> Self {
        self.visitor = Arc::new(visitor);
        self
    }

    pub fn is_fatal_error(
        mut self,
        is_fatal_error: impl Fn(&M::Error) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.is_fatal_error = Arc::new(is_fatal_error);
        self
    }

    pub fn map_state(
        mut self,
        map_state: impl Fn(M::State) -> Option<S> + Send + Sync + 'static,
    ) -> Self {
        self.map_state = Arc::new(map_state);
        self
    }

    pub fn map_action(
        mut self,
        map_action: impl Fn(M::Action) -> Option<A> + Send + Sync + 'static,
    ) -> Self {
        self.map_action = Arc::new(map_action);
        self
    }

    pub fn run_terminal(self) -> Result<TerminalSet<S>, M::Error> {
        let (_report, _, terminals) = traverse(self, false, true)?;
        Ok(terminals.unwrap())
    }

    pub fn graph(self) -> Result<DiGraph<S, A>, M::Error> {
        let (_report, graph, _) = traverse(self, true, false)?;
        Ok(graph.unwrap())
    }
}

impl<M, S, A> Traversal<M, S, A>
where
    M: Machine,
    S: 'static + Clone + Debug + Eq + Hash,
    A: 'static + Clone + Debug,
    M::State: 'static + Clone + Debug + Eq + Hash,
    M::Action: 'static + Clone + Debug,
    M::Error: 'static,
{
    pub fn specced<P>(
        self,
        props: P,
        ltl: &str,
    ) -> anyhow::Result<Traversal<ModelChecker<M, P>, ModelCheckerState<S, M::Action>, A>>
    where
        P: PropMapping + Send + Sync + 'static,
        Transition<M>: Propositions<P::Prop>,
    {
        let machine = ModelChecker::new(self.machine, props, ltl)?;
        let initial = self
            .initial
            .into_iter()
            .map(|s| machine.initial(s))
            .collect();
        let visitor = self.visitor;
        let map_state = self.map_state;
        let map_action = self.map_action;
        Ok(Traversal {
            machine,
            initial,
            max_depth: self.max_depth,
            trace_every: self.trace_every,
            trace_errors: self.trace_errors,
            ignore_loopbacks: self.ignore_loopbacks,
            visitor: Arc::new(move |s, visit| {
                visitor(&*s, visit).map_err(ModelCheckerTransitionError::MachineError)
            }),
            is_fatal_error: Arc::new(|e| {
                !matches!(e, ModelCheckerTransitionError::MachineError(_))
            }),
            map_state: Arc::new(move |s| s.map_state(|ss| (map_state)(ss))),
            map_action: Arc::new(move |a| (map_action)(a)),
        })
    }
}

impl<M, S, A, P> Traversal<ModelChecker<M, P>, ModelCheckerState<S, M::Action>, A>
where
    M: Machine + Send + Sync + 'static,
    M::State: Clone + Debug + Eq + Hash + Send + Sync + 'static,
    S: Clone + Debug + Eq + Hash + Send + Sync + 'static,
    M::Action: Clone + Debug + Eq + Hash + Exhaustive + Send + Sync + 'static,
    A: Clone + Debug + Eq + Hash + Exhaustive + Send + Sync + 'static,
    M::Error: Debug + Send + Sync + 'static,
    P: PropMapping + Send + Sync + 'static,
    Transition<M>: Propositions<P::Prop>,
{
    pub fn model_check(self) -> Result<TraversalReport, ModelCheckerError<M>> {
        match traverse(self, true, false) {
            Ok((report, graph, _)) => {
                let condensed = petgraph::algo::condensation(graph.unwrap(), true);

                let leaves = condensed.node_indices().filter(|n| {
                    let outgoing = condensed
                        .neighbors_directed(*n, petgraph::Direction::Outgoing)
                        .count();
                    outgoing == 0
                });

                for index in leaves {
                    let scc = condensed.node_weight(index).unwrap();
                    let accepting = scc.iter().any(|n| n.buchi.is_accepting());
                    if !accepting {
                        let mut paths = scc.iter().map(|n| n.pathstate.path.clone()).collect_vec();
                        paths.sort_by_key(|p| p.len());
                        return Err(ModelCheckerError::Liveness { paths });
                    }
                }

                Ok(report)
            }
            Err(e) => match e {
                ModelCheckerTransitionError::BuchiError(e) => Err(ModelCheckerError::Safety {
                    path: e.path,
                    states: e.states,
                }),
                ModelCheckerTransitionError::MachineError(e) => {
                    unreachable!("{e:?}");
                }
            },
        }
    }

    pub fn model_check_report(self) {
        model_checker_report(self.model_check());
    }
}

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

pub fn traverse<M, S, A>(
    traversal: Traversal<M, S, A>,
    do_graphing: bool,
    record_terminals: bool,
) -> Result<
    (
        TraversalReport,
        Option<DiGraph<S, A>>,
        Option<TerminalSet<S>>,
    ),
    M::Error,
>
where
    M: Machine + Send + Sync + 'static,
    M::State: Cog + 'static,
    M::Action: Exhaustive + Cog + 'static,
    M::Error: Debug + Send + Sync + 'static,
    S: Cog + 'static,
    A: Cog + 'static,
{
    let machine = traversal.machine;
    let initial = traversal.initial;
    let max_depth = traversal.max_depth;
    let trace_every = traversal.trace_every;
    let ignore_loopbacks = traversal.ignore_loopbacks;
    let visitor = traversal.visitor;
    let is_fatal_error = traversal.is_fatal_error;
    let trace_errors = traversal.trace_errors;
    let map_state = traversal.map_state;
    let map_action = traversal.map_action;

    let terminals: Arc<Mutex<TerminalSet<S>>> = Arc::new(Mutex::new(HashSet::new()));
    // let loop_terminals: Arc<Mutex<TerminalSet<S>>> = Arc::new(Mutex::new(HashSet::new()));
    let visited_states: Arc<Mutex<HashMap<S, NodeIndex>>> = Arc::new(Mutex::new(HashMap::new()));
    let visited_edges: Arc<Mutex<HashSet<(NodeIndex, NodeIndex, A)>>> =
        Arc::new(Mutex::new(HashSet::new()));

    let graph = Arc::new(Mutex::new(DiGraph::new()));

    let seen = Arc::new(crossbeam::queue::SegQueue::new());

    let (err_tx, err_rx) = crossbeam::channel::bounded(1);

    for state in initial {
        seen.push((state, Option::<(NodeIndex, M::Action)>::None, 0));
    }

    let stop = Arc::new(AtomicBool::new(false));
    let total_steps = Arc::new(AtomicUsize::new(0));
    let num_seen = Arc::new(AtomicUsize::new(0));
    let num_terminations = Arc::new(AtomicUsize::new(0));
    let num_edges_skipped = Arc::new(AtomicUsize::new(0));
    let max_depth_seen = Arc::new(AtomicUsize::new(0));

    let all_actions: im::Vector<_> = M::Action::iter_exhaustive(None).collect();

    let start_time = std::time::Instant::now();

    let task = {
        let graph = graph.clone();
        let stop = stop.clone();
        let total_steps = total_steps.clone();
        let num_seen = num_seen.clone();
        let num_terminations = num_terminations.clone();
        let num_edges_skipped = num_edges_skipped.clone();
        let max_depth_seen = max_depth_seen.clone();
        let visited = visited_states.clone();
        let terminals = terminals.clone();
        // let loop_terminals = loop_terminals.clone();

        let active_threads = AtomicUsize::new(0);
        let prev_trace = Mutex::new(IterTrace::default());
        let trace_every = trace_every.unwrap_or(usize::MAX);

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
                        "iter={:<5} │ visited={:<8} Δ={:<8} │ queued={:<8} Δ={:<8} │ {}",
                        trace.iter.human_count_bare().to_string(),
                        trace.visited.human_count_bare().to_string(),
                        visited_diff_str,
                        trace.queued.human_count_bare().to_string(),
                        queued_diff_str,
                        depth_str,
                    );
                    *prev = trace;
                }

                let mapped_state = if let Some(mapped_state) = map_state(state.clone()) {
                    mapped_state
                } else {
                    // skip a node with no mapping
                    continue;
                };

                let (already_seen, node_ix) = {
                    let mut visited = visited.lock();
                    match visited.entry(mapped_state.clone()) {
                        std::collections::hash_map::Entry::Occupied(entry) => (true, *entry.get()),
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            max_depth_seen.fetch_max(depth, SeqCst);

                            if do_graphing {
                                let node_ix = graph.lock().add_node(mapped_state.clone());
                                entry.insert(node_ix);
                                (false, node_ix)
                            } else {
                                entry.insert(NodeIndex::end());
                                (false, NodeIndex::end())
                            }
                        }
                    }
                };

                if do_graphing {
                    let mut graph = graph.lock();
                    if let Some((prev_node_ix, edge)) = prev_node {
                        if let Some(edge) = map_action(edge) {
                            let ignore = ignore_loopbacks && prev_node_ix == node_ix;
                            if !ignore
                                && visited_edges.lock().insert((
                                    prev_node_ix,
                                    node_ix,
                                    edge.clone(),
                                ))
                            {
                                let _ = graph.add_edge(prev_node_ix, node_ix, edge);
                            }
                        }
                    }
                }

                // Don't explore the same node twice
                if already_seen {
                    //     visitor(&state, VisitType::LoopTerminal)?;
                    //     if record_terminals {
                    //         loop_terminals.lock().insert(state);
                    //     }

                    continue;
                }

                // If this is a terminal state, no need to explore further.
                // TODO: should also check if terminal due to no outgoing actions
                if machine.is_terminal(&state) {
                    num_terminations.fetch_add(1, SeqCst);
                    visitor(&state, VisitType::Terminal)?;
                    if record_terminals {
                        terminals.lock().insert(mapped_state);
                    }
                    continue;
                } else {
                    visitor(&state, VisitType::Normal)?;
                }

                // Respect the depth limit
                if depth >= max_depth.unwrap_or(usize::MAX) {
                    continue;
                }

                // Queue up visits to all nodes reachable from this node..
                for action in all_actions.iter() {
                    let prev_node = if do_graphing {
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

                            if is_fatal_error(&err) {
                                return Err(err);
                            }
                            if trace_errors {
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
        max_depth: max_depth_seen.load(SeqCst),
    };
    let terminals = record_terminals.then(|| {
        Arc::into_inner(terminals).unwrap().into_inner()
        // Arc::into_inner(loop_terminals).unwrap().into_inner(),
    });
    let graph = do_graphing.then_some(Arc::into_inner(graph).unwrap().into_inner());
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
