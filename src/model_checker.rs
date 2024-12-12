pub mod buchi;

mod check;
#[cfg(test)]
mod tests;

use std::{fmt::Debug, hash::Hash};

use buchi::*;

use crate::logic::Propositions;
use crate::machine::{
    store_path::{StorePathMachine, StorePathState},
    Machine, TransitionResult,
};

pub struct ModelChecker<M>
where
    M: Machine,
{
    buchi: BuchiAutomaton<M::State>,
    machine: StorePathMachine<M>,
}

/*                                   █████       ███
                                   ░░███       ░░░
 █████████████    ██████    ██████  ░███████   ████  ████████    ██████
░░███░░███░░███  ░░░░░███  ███░░███ ░███░░███ ░░███ ░░███░░███  ███░░███
 ░███ ░███ ░███   ███████ ░███ ░░░  ░███ ░███  ░███  ░███ ░███ ░███████
 ░███ ░███ ░███  ███░░███ ░███  ███ ░███ ░███  ░███  ░███ ░███ ░███░░░
 █████░███ █████░░████████░░██████  ████ █████ █████ ████ █████░░██████
░░░░░ ░░░ ░░░░░  ░░░░░░░░  ░░░░░░  ░░░░ ░░░░░ ░░░░░ ░░░░ ░░░░░  ░░░░░░   */

impl<M> Machine for ModelChecker<M>
where
    M: Machine,
    M::State: Propositions + Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    type State = ModelCheckerState<M>;
    type Action = M::Action;
    type Error = ModelCheckerTransitionError<M>;
    type Fx = M::Fx;

    fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        let ModelCheckerState { state, buchi } = state;

        let buchi_next = self
            .buchi
            .transition_(buchi, state.state.clone())
            .map_err(|error| {
                ModelCheckerTransitionError::BuchiError(ModelCheckerBuchiError {
                    error,
                    path: state.path.clone(),
                })
            })?;

        let (next, fx) = self
            .machine
            .transition(state, action)
            .map_err(ModelCheckerTransitionError::MachineError)?;

        let next = ModelCheckerState {
            state: next,
            buchi: buchi_next.into(),
        };
        Ok((next, fx))
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        self.machine.is_terminal(&state.state)
    }
}

impl<M> ModelChecker<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    pub fn new(machine: M, ltl: &str) -> Self {
        let buchi = BuchiAutomaton::from_ltl(ltl);
        Self {
            buchi,
            machine: StorePathMachine::from(machine),
        }
    }

    pub fn initial(&self, state: M::State) -> ModelCheckerState<M> {
        let inits = self
            .buchi
            .states
            .keys()
            .cloned()
            .filter(|name| name.ends_with("_init"));

        ModelCheckerState::new(state, inits)
    }
}

#[derive(derive_bounded::Debug)]
#[bounded_to(M::Action, M::Error)]
pub enum ModelCheckerTransitionError<M: Machine>
where
    M::Action: Clone,
{
    BuchiError(ModelCheckerBuchiError<M>),
    MachineError(M::Error),
}

#[derive(derive_bounded::Debug)]
#[bounded_to(M::Action)]
pub struct ModelCheckerBuchiError<M: Machine>
where
    M::Action: Clone,
{
    error: BuchiError,
    path: im::Vector<M::Action>,
}

/*        █████               █████
         ░░███               ░░███
  █████  ███████    ██████   ███████    ██████
 ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
░░█████   ░███      ███████   ░███    ░███████
 ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
 ██████   ░░█████ ░░████████  ░░█████ ░░██████
░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░  */

#[derive(
    derive_more::Debug,
    derive_bounded::Clone,
    derive_bounded::PartialEq,
    derive_bounded::Eq,
    derive_more::Deref,
)]
#[bounded_to(StorePathState<M>)]
pub struct ModelCheckerState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    #[deref]
    state: StorePathState<M>,
    #[debug(skip)]
    buchi: BuchiPaths,
}

// NB: regrettably we can't easily derive Hash because ModelChecker is not Hash,
//     even though that doesn't matter.
// TODO: make PR to derive_bounded to support Hash?
impl<M> Hash for ModelCheckerState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.state.state.hash(state);
        self.buchi.hash(state);
    }
}

impl<M> ModelCheckerState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    pub fn new(state: M::State, buchi_states: impl IntoIterator<Item = StateName>) -> Self {
        Self {
            state: StorePathState::new(state),
            buchi: BuchiPaths(buchi_states.into_iter().collect()),
        }
    }
}
