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
    dfa::checked::{Checker, CheckerError, CheckerState},
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

pub fn traverse_checked<M>(
    machine: Checker<M>,
    initial: CheckerState<M::State, M::Action>,
) -> Result<(), CheckerError<M::Action, M::Error>>
where
    M: Machine,
    M::State: Clone + Eq + Hash + Debug,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
{
    let config = TraversalConfig::default().stop_on_checker_error();
    traverse(machine, initial, &config)
}

pub fn traverse<M>(
    machine: M,
    initial: M::State,
    config: &TraversalConfig<M::Error>,
) -> Result<(), M::Error>
where
    M: Machine,
    M::State: Clone + Eq + Hash,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
{
    let mut visited: HashSet<M::State> = HashSet::new();
    let mut to_visit: VecDeque<(M::State, usize, im::Vector<M::Action>)> = VecDeque::new();

    to_visit.push_back((initial, 0, im::vector![]));

    let mut total_steps = 0;
    let mut num_errors = 0;
    let mut num_terminations = 0;
    let mut num_iters = 0;

    while let Some((state, distance, path)) = to_visit.pop_front() {
        num_iters += 1;
        if num_iters % 1000 == 0 {
            tracing::debug!("iter {num_iters}");
        }
        if config.max_iters.map(|m| num_iters >= m).unwrap_or(false) {
            panic!("max iters of {} reached", config.max_iters.unwrap());
        }

        // Don't explore the same node twice, and respect the depth limit
        if distance > config.max_depth.unwrap_or(usize::MAX) || visited.contains(&state) {
            continue;
        }

        visited.insert(state.clone());

        // If this is a terminal state, no need to explore further.
        if machine.is_terminal(&state) {
            num_terminations += 1;
            continue;
        }

        // Queue up visits to all nodes reachable from this node..
        for action in M::Action::iter_exhaustive(config.max_actions) {
            total_steps += 1;
            match machine.transition(state.clone(), action.clone()).map(first) {
                Ok(node) => {
                    let mut path = path.clone();
                    path.push_back(action);
                    to_visit.push_back((node, distance + 1, path));
                }
                Err(err) => {
                    num_errors += 1;
                    if let Some(ref is_fatal_error) = config.is_fatal_error {
                        if is_fatal_error(&err) {
                            return Err(err);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
