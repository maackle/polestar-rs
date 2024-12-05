use exhaustive::Exhaustive;
use im::Vector;
use parking_lot::Mutex;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::{
    machine::checked::{CheckerError, CheckerMachine, CheckerState, PredicateError},
    util::first,
    Machine,
};

#[derive(Clone)]
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

pub fn traverse_checked<M>(
    machine: CheckerMachine<M>,
    initial: CheckerState<M>,
    config: TraversalConfig<CheckerMachine<M>>,
) -> Result<TraversalReport, PredicateError<M>>
where
    M: Machine + Debug + Send + Sync,
    M::State: Clone + Eq + Hash + Debug + Send + Sync,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug + Send + Sync,
    M::Error: Debug + Send + Sync,
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
    let (report, _) = traverse(machine, initial, &config).map_err(|e| match e {
        CheckerError::Predicate(e) => e,
        CheckerError::Machine(_) => unreachable!(),
    })?;

    Ok(report)
}

pub fn traverse<M>(
    machine: M,
    initial: M::State,
    config: &TraversalConfig<M>,
) -> Result<
    (
        TraversalReport,
        Option<(TerminalSet<M::State>, TerminalSet<M::State>)>,
    ),
    M::Error,
>
where
    M: Machine + Send + Sync,
    M::State: Clone + Eq + Hash + Debug + Send + Sync,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug + Send + Sync,
    M::Error: Debug + Send + Sync,
{
    use rayon::iter::*;

    let machine = Arc::new(machine);

    let mut terminals: TerminalSet<M::State> = HashSet::new();
    let mut loop_terminals: TerminalSet<M::State> = HashSet::new();

    let mut visited: HashMap<M::State, ()> = HashMap::new();

    let to_visit: Mutex<VecDeque<(M::State, usize)>> = Mutex::new(VecDeque::new());

    to_visit.lock().push_back((initial, 0));

    let mut report = TraversalReport::default();

    let all_actions: im::Vector<_> = M::Action::iter_exhaustive(config.max_actions).collect();

    while let Some((state, depth)) = {
        let mut lock = to_visit.lock();
        lock.pop_front()
    } {
        report.total_steps += 1;
        if report.total_steps % config.trace_every == 0 {
            tracing::info!(
                "iter={}, to_visit={}, visited={}, depth={}",
                report.total_steps,
                to_visit.lock().len(),
                visited.len(),
                depth
            );
        }
        if config
            .max_iters
            .map(|m| report.total_steps >= m)
            .unwrap_or(false)
        {
            panic!("max iters of {} reached", config.max_iters.unwrap());
        }

        let already_seen = visited.insert(state.clone(), ()).is_some();

        // Don't explore the same node twice, and respect the depth limit
        if already_seen || depth > config.max_depth.unwrap_or(usize::MAX) {
            if let Some(ref on_terminal) = config.visitor {
                on_terminal(&state, VisitType::LoopTerminal)?;
            }
            if config.record_terminals {
                loop_terminals.insert(state);
            }
            continue;
        }
        // If this is a terminal state, no need to explore further.
        else if machine.is_terminal(&state) {
            report.num_terminations += 1;
            if let Some(ref on_terminal) = config.visitor {
                on_terminal(&state, VisitType::Terminal)?;
            }
            if config.record_terminals {
                terminals.insert(state);
            }
            continue;
        } else {
            if let Some(ref on_terminal) = config.visitor {
                on_terminal(&state, VisitType::Normal)?;
            }
        }

        // Queue up visits to all nodes reachable from this node..
        all_actions
            .par_iter()
            .cloned()
            .map(|action| {
                match machine.transition(state.clone(), action.clone()).map(first) {
                    Ok(node) => {
                        to_visit.lock().push_back((node, depth + 1));
                    }
                    Err(err) => {
                        // report.num_errors += 1;
                        tracing::debug!("traversal error: {:?}", err);
                        if let Some(ref is_fatal_error) = config.is_fatal_error {
                            if is_fatal_error(&err) {
                                return Err(err);
                            }
                        }
                    }
                }
                Ok(())
            })
            .collect::<Result<Vec<_>, _>>()?;
    }
    report.num_visited = visited.len();
    let terminals = config.record_terminals.then(|| (terminals, loop_terminals));
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
