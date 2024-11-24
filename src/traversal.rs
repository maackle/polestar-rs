use exhaustive::Exhaustive;
use petgraph::graph::{DiGraph, NodeIndex};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    hash::Hash,
    path::Path,
    sync::Arc,
};

use crate::{
    dfa::checked::{Checker, CheckerError, CheckerState, PredicateError},
    util::first,
    Machine,
};

#[derive(Clone)]
pub struct TraversalConfig<E> {
    pub max_actions: Option<usize>,
    pub max_depth: Option<usize>,
    pub max_iters: Option<usize>,
    pub ignore_loopbacks: bool,
    pub is_fatal_error: Option<Arc<dyn Fn(&E) -> bool>>,
}

impl<E> Default for TraversalConfig<E> {
    fn default() -> Self {
        Self {
            max_actions: None,
            max_depth: None,
            max_iters: None,
            ignore_loopbacks: false,
            is_fatal_error: None,
        }
    }
}

impl<A: Clone, E> TraversalConfig<CheckerError<A, E>> {
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
    initial: CheckerState<M::State, M::Action>,
) -> Result<TraversalReport, CheckerError<M::Action, M::Error>>
where
    M: Machine,
    M::State: Clone + Eq + Hash + Debug,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
{
    let config = TraversalConfig::default().stop_on_checker_error();
    let (terminals, report) = traverse(machine, initial, &config)?;
    if terminals.is_empty() {
        return Err(CheckerError::Predicate(PredicateError {
            error: "no states visited".into(),
            path: Default::default(),
        }));
    }
    let results: Vec<_> = terminals.into_iter().map(|s| s.finalize()).collect();
    if results.iter().all(|r| r.is_err()) {
        let mut errors: Vec<_> = results
            .into_iter()
            .filter_map(|r| match r {
                Ok(_) => None,
                Err(e) => Some(e),
            })
            .collect();
        errors.sort_by_key(|(e, p)| p.len());
        let (error, path) = errors.pop().unwrap();
        return Err(CheckerError::Predicate(PredicateError { error, path }));
    }

    Ok(report)
}

pub fn traverse<M>(
    machine: M,
    initial: M::State,
    config: &TraversalConfig<M::Error>,
) -> Result<(HashSet<M::State>, TraversalReport), M::Error>
where
    M: Machine,
    M::State: Clone + Eq + Hash,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
{
    let mut visited: HashSet<M::State> = HashSet::new();
    let mut terminals: HashSet<M::State> = HashSet::new();
    let mut to_visit: VecDeque<(M::State, usize, im::Vector<M::Action>)> = VecDeque::new();

    to_visit.push_back((initial, 0, im::vector![]));

    let mut report = TraversalReport::default();

    while let Some((state, distance, path)) = to_visit.pop_front() {
        report.total_steps += 1;
        if report.total_steps % 1000 == 0 {
            tracing::debug!("iter {}", report.total_steps);
        }
        if config
            .max_iters
            .map(|m| report.total_steps >= m)
            .unwrap_or(false)
        {
            panic!("max iters of {} reached", config.max_iters.unwrap());
        }

        // Don't explore the same node twice, and respect the depth limit
        if distance > config.max_depth.unwrap_or(usize::MAX) || visited.contains(&state) {
            terminals.insert(state);
            continue;
        }

        visited.insert(state.clone());

        // If this is a terminal state, no need to explore further.
        if machine.is_terminal(&state) {
            report.num_terminations += 1;
            continue;
        }

        // Queue up visits to all nodes reachable from this node..
        for action in M::Action::iter_exhaustive(config.max_actions) {
            match machine.transition(state.clone(), action.clone()).map(first) {
                Ok(node) => {
                    let mut path = path.clone();
                    path.push_back(action);
                    to_visit.push_back((node, distance + 1, path));
                }
                Err(err) => {
                    report.num_errors += 1;
                    if let Some(ref is_fatal_error) = config.is_fatal_error {
                        if is_fatal_error(&err) {
                            return Err(err);
                        }
                    }
                }
            }
        }
    }
    report.num_visited = visited.len();
    Ok((terminals, report))
}

#[cfg(test)]
mod tests {
    use crate::{dfa::checked::Predicate, MachineResult};

    use super::*;

    #[test]
    fn test_checked_traversal() {
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

            fn transition(&self, state: Self::State, action: Self::Action) -> MachineResult<Self> {
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
        let checker = SimpleMachine
            .checked()
            .predicate(P::always(big.clone().implies(P::next(P::not(big)))))
            // .predicate(P::always(divby(2).or(divby(3))))
            .predicate(P::eventually(divby(3)));

        let err = checker
            .check_fold(1, [Action::Double, Action::Double])
            .unwrap_err();

        assert!(err.unwrap_predicate().error.contains("div-by-3"));

        let initial = checker.initial(1);
        let report = traverse_checked(checker, initial).unwrap();
        dbg!(report);
    }
}
