use exhaustive::Exhaustive;
use im::Vector;
use parking_lot::Mutex;

use std::{
    collections::{HashSet, VecDeque},
    fmt::Debug,
    hash::Hash,
    sync::Arc,
};

use crate::{
    machine::checked::{Checker, CheckerError, CheckerState, PredicateError},
    util::first,
    Machine,
};

#[derive(Clone)]
pub struct TraversalConfig<E: Send + Sync> {
    pub max_actions: Option<usize>,
    pub max_depth: Option<usize>,
    pub max_iters: Option<usize>,
    pub trace_every: usize,
    pub ignore_loopbacks: bool,
    pub trace_error: bool,
    pub is_fatal_error: Option<Arc<dyn Fn(&E) -> bool + Send + Sync>>,
}

impl<E: Send + Sync> Default for TraversalConfig<E> {
    fn default() -> Self {
        Self {
            max_actions: None,
            max_depth: None,
            max_iters: None,
            trace_every: 1000,
            ignore_loopbacks: false,
            trace_error: false,
            is_fatal_error: None,
        }
    }
}

impl<A: Clone + Send + Sync, E: Send + Sync> TraversalConfig<CheckerError<A, E>> {
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
    machine: Checker<M>,
    initial: CheckerState<M>,
    config: TraversalConfig<CheckerError<M::Action, M::Error>>,
) -> Result<TraversalReport, PredicateError<M::Action>>
where
    M: Machine + Debug + Send + Sync,
    M::State: Clone + Eq + Hash + Debug + Send + Sync,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug + Send + Sync,
    M::Error: Debug + Send + Sync,
{
    let config = config.stop_on_checker_error();
    let (terminals, report) = traverse(machine, initial, &config).map_err(|e| match e {
        CheckerError::Predicate(e) => e,
        CheckerError::Machine(_) => unreachable!(),
    })?;
    if terminals.is_empty() {
        return Err(PredicateError {
            error: "no states visited".into(),
            path: Default::default(),
        });
    }
    let results: Vec<_> = terminals.into_iter().map(|(s, _)| s.finalize()).collect();
    if results.iter().all(|r| r.is_err()) {
        let mut errors: Vec<_> = results
            .into_iter()
            .filter_map(|r| match r {
                Ok(_) => None,
                Err(e) => Some(e),
            })
            .collect();
        errors.sort_by_key(|(_, p)| p.len());
        let (error, path) = errors.pop().unwrap();
        return Err(PredicateError { error, path });
    }

    Ok(report)
}

pub fn traverse<M>(
    machine: M,
    initial: M::State,
    config: &TraversalConfig<M::Error>,
) -> Result<(HashSet<(M::State, im::Vector<M::Action>)>, TraversalReport), M::Error>
where
    M: Machine + Send + Sync,
    M::State: Clone + Eq + Hash + Debug + Send + Sync,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug + Send + Sync,
    M::Error: Debug + Send + Sync,
{
    use rayon::iter::*;

    let machine = Arc::new(machine);

    let mut visited: HashSet<M::State> = HashSet::new();
    let mut terminals: HashSet<(M::State, im::Vector<M::Action>)> = HashSet::new();
    let to_visit: Mutex<VecDeque<(M::State, usize, im::Vector<M::Action>)>> =
        Mutex::new(VecDeque::new());

    to_visit.lock().push_back((initial, 0, im::vector![]));

    let mut report = TraversalReport::default();

    let all_actions: im::Vector<_> = M::Action::iter_exhaustive(config.max_actions).collect();

    while let Some((state, depth, path)) = {
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

        // Don't explore the same node twice, and respect the depth limit
        if depth > config.max_depth.unwrap_or(usize::MAX) || visited.contains(&state) {
            terminals.insert((state, path));
            continue;
        }

        visited.insert(state.clone());

        // If this is a terminal state, no need to explore further.
        if machine.is_terminal(&state) {
            report.num_terminations += 1;
            terminals.insert((state, path));
            continue;
        }

        // Queue up visits to all nodes reachable from this node..
        all_actions
            .par_iter()
            .cloned()
            .map(|action| {
                match machine.transition(state.clone(), action.clone()).map(first) {
                    Ok(node) => {
                        let mut path = path.clone();
                        path.push_back(action);
                        to_visit.lock().push_back((node, depth + 1, path));
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
    Ok((terminals, report))
}

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
        let report = traverse_checked(
            checker,
            initial,
            TraversalConfig {
                ..Default::default()
            },
        )
        .unwrap();
        dbg!(report);
    }
}
