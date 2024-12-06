use exhaustive::Exhaustive;
use parking_lot::Mutex;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc,
    },
};

use crate::{
    machine::checked::{CheckerError, CheckerMachine, CheckerState, PredicateError},
    util::first,
    Machine,
};

#[derive(derive_bounded::Clone)]
#[bounded_to(M::State)]
pub struct TraversalConfig<M: Machine> {
    pub max_actions: Option<usize>,
    pub max_depth: Option<usize>,
    pub max_iters: Option<usize>,
    pub trace_every: usize,
    pub ignore_loopbacks: bool,
    pub record_terminals: bool,
    pub trace_error: bool,
    pub visitor: Option<Arc<dyn Fn(&M::State, VisitType) -> Result<(), M::Error> + Send + Sync>>,
    pub is_fatal_error: Option<Arc<dyn Fn(&M::Error) -> bool + Send + Sync>>,
}

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
            trace_every: 1000,
            ignore_loopbacks: false,
            record_terminals: false,
            trace_error: false,
            visitor: None,
            is_fatal_error: None,
        }
    }
}

impl<M: Machine> TraversalConfig<CheckerMachine<M>>
where
    M: Machine + Debug,
    M::State: Clone + Debug,
    M::Action: Clone + Debug,
{
    pub fn stop_on_checker_error(mut self) -> Self {
        self.is_fatal_error = Some(Arc::new(|err| matches!(err, CheckerError::Predicate(_))));
        self
    }
}

#[derive(Debug, Default)]
pub struct TraversalReport {
    pub num_visited: usize,
    pub num_terminations: usize,
    pub num_errors: usize,
    pub total_steps: usize,
    pub time_taken: std::time::Duration,
}

// impl Default for TraversalReport {
//     fn default() -> Self {
//         Self {
//             num_states: 0,
//             num_terminations: 0,
//             num_errors: 0,
//             total_steps: 0,
//         }
//     }
// }

pub fn traverse_checked<M, S>(
    machine: CheckerMachine<M>,
    initial: CheckerState<M>,
    config: TraversalConfig<CheckerMachine<M>>,
    map_state: impl Fn(M::State) -> Option<S> + Send + Sync + 'static,
) -> Result<TraversalReport, PredicateError<M>>
where
    M: Machine + Debug + Send + Sync + 'static,
    M::State: Clone + Eq + Hash + Debug + Send + Sync + 'static,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug + Send + Sync + 'static,
    M::Error: Debug + Send + Sync + 'static,
    S: Clone + Eq + Hash + Debug + Send + Sync + 'static,
{
    let mut config = config.stop_on_checker_error();

    config.visitor = Some(Arc::new(|s, visit_type| {
        if matches!(visit_type, VisitType::Terminal | VisitType::LoopTerminal) {
            s.clone().finalize().map_err(|error| {
                CheckerError::Predicate(PredicateError {
                    error,
                    path: s.state.path.clone(),
                })
            })
        } else {
            Ok(())
        }
    }));

    let (report, _) = traverse(machine, initial, &config, move |s| map_state(s.state.state))
        .map_err(|e| match e {
            CheckerError::Predicate(e) => e,
            CheckerError::Machine(_) => unreachable!(),
        })?;

    Ok(report)
}

pub fn traverse<M, S>(
    machine: M,
    initial: M::State,
    config: &TraversalConfig<M>,
    map_state: impl Fn(M::State) -> Option<S> + Send + Sync + 'static,
) -> Result<
    (
        TraversalReport,
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
    let machine = Arc::new(machine);

    let terminals: Arc<Mutex<TerminalSet<M::State>>> = Arc::new(Mutex::new(HashSet::new()));
    let loop_terminals: Arc<Mutex<TerminalSet<M::State>>> = Arc::new(Mutex::new(HashSet::new()));
    let visited: Arc<Mutex<HashMap<S, ()>>> = Arc::new(Mutex::new(HashMap::new()));

    let seen = Arc::new(crossbeam::queue::SegQueue::new());

    let (err_tx, err_rx) = crossbeam::channel::bounded(1);

    seen.push((initial, 0));
    // to_visit.lock().push_back((initial, 0));

    let stop = Arc::new(AtomicBool::new(false));
    let total_steps = Arc::new(AtomicUsize::new(0));
    let num_seen = Arc::new(AtomicUsize::new(0));
    let num_terminations = Arc::new(AtomicUsize::new(0));
    let num_errors = Arc::new(AtomicUsize::new(0));

    let all_actions: im::Vector<_> = M::Action::iter_exhaustive(config.max_actions).collect();

    let config: TraversalConfig<M> = config.clone();

    let start_time = std::time::Instant::now();

    let task = {
        let stop = stop.clone();
        let total_steps = total_steps.clone();
        let num_seen = num_seen.clone();
        let num_terminations = num_terminations.clone();
        let num_errors = num_errors.clone();
        let visited = visited.clone();
        let terminals = terminals.clone();
        let loop_terminals = loop_terminals.clone();

        move |thread_index: usize| {
            if thread_index > 0 {
                // Give the seen queue time to fill up
                // XXX: this is a hack to avoid starvation, could be more robust
                while seen.len() < 1000 {
                    if stop.load(std::sync::atomic::Ordering::SeqCst) {
                        // The first thread already completed all work
                        return Ok(());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }

            while let Some((state, depth)) = seen.pop() {
                if stop.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
                let iter = total_steps.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if iter % config.trace_every == 0 {
                    tracing::info!(
                        "iter={iter}, seen={}, visited={}, depth={}",
                        num_seen.load(std::sync::atomic::Ordering::SeqCst),
                        visited.lock().len(),
                        depth
                    );
                }
                if config.max_iters.map(|m| iter >= m).unwrap_or(false) {
                    panic!("max iters of {} reached", config.max_iters.unwrap());
                }

                let already_seen = if let Some(mapped) = map_state(state.clone()) {
                    visited.lock().insert(mapped, ()).is_some()
                } else {
                    // skip a node with no mapping
                    continue;
                };

                // Don't explore the same node twice, and respect the depth limit
                if already_seen || depth > config.max_depth.unwrap_or(usize::MAX) {
                    if let Some(ref on_terminal) = config.visitor {
                        on_terminal(&state, VisitType::LoopTerminal)?
                    }
                    if config.record_terminals {
                        loop_terminals.lock().insert(state);
                    }
                    continue;
                }
                // If this is a terminal state, no need to explore further.
                else if machine.is_terminal(&state) {
                    num_terminations.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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

                // Queue up visits to all nodes reachable from this node..
                for action in all_actions.iter().cloned() {
                    match machine.transition(state.clone(), action.clone()).map(first) {
                        Ok(node) => {
                            num_seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            seen.push((node, depth + 1));
                        }
                        Err(err) => {
                            num_errors.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            tracing::trace!("traversal error: {:?}", err);
                            if let Some(ref is_fatal_error) = config.is_fatal_error {
                                if is_fatal_error(&err) {
                                    return Err(err);
                                }
                            }
                        }
                    }
                }
            }
            tracing::info!("traversal thread {} done", thread_index);

            // handle the case where the first thread rips through all work
            // before any other threads even get started
            if thread_index == 0 {
                stop.store(true, std::sync::atomic::Ordering::SeqCst);
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
                stop.store(true, std::sync::atomic::Ordering::SeqCst);
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
        num_visited: visited.lock().len(),
        num_terminations: num_terminations.load(std::sync::atomic::Ordering::SeqCst),
        num_errors: num_errors.load(std::sync::atomic::Ordering::SeqCst),
        total_steps: total_steps.load(std::sync::atomic::Ordering::SeqCst),
    };
    let terminals = config.record_terminals.then(|| {
        (
            Arc::into_inner(terminals).unwrap().into_inner(),
            Arc::into_inner(loop_terminals).unwrap().into_inner(),
        )
    });
    Ok((report, terminals))
}

pub type TerminalSet<S> = HashSet<S>;

#[cfg(test)]
mod tests {
    use crate::{machine::checked::Predicate, TransitionResult};

    use super::*;

    #[test]
    fn test_checked_traversal() {
        #[derive(Clone, Debug)]
        struct SimpleMachine;
        use Predicate as P;

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
