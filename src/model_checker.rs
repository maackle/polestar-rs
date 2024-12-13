pub mod buchi;

mod check;
pub use check::*;
#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::{fmt::Debug, hash::Hash};

use buchi::*;

use crate::logic::{Pair, PropMapping, Propositions};
use crate::machine::{
    store_path::{StorePathMachine, StorePathState},
    Machine, TransitionResult,
};

pub struct ModelChecker<M, P>
where
    M: Machine,
    P: PropMapping,
{
    buchi: BuchiAutomaton<M::State, P>,
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

impl<M, P> Machine for ModelChecker<M, P>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
    P: PropMapping,
    // TODO: if a proc macro is ever written, make it clearer that you must implement Propositions for pairs, not just the state.
    //       (or somehow make this easier)
    Pair<M::State>: Propositions<P::Prop>,
{
    type State = ModelCheckerState<M::State, M::Action>;
    type Action = M::Action;
    type Error = ModelCheckerTransitionError<M>;
    type Fx = M::Fx;

    fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        let ModelCheckerState {
            pathstate: state,
            buchi,
        } = state;

        let prev = state.state.clone();

        let (next, fx) = self
            .machine
            .transition(state.clone(), action)
            .map_err(ModelCheckerTransitionError::MachineError)?;

        let buchi_next = self
            .buchi
            .transition_(buchi, (prev.clone(), next.state.clone()))
            .map_err(|error| {
                ModelCheckerTransitionError::BuchiError(ModelCheckerBuchiError {
                    error,
                    path: state.path.clone(),
                    states: (prev, next.state.clone()),
                })
            })?;

        let next = ModelCheckerState {
            pathstate: next,
            buchi: buchi_next.into(),
        };
        Ok((next, fx))
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        self.machine.is_terminal(&state.pathstate)
    }
}

impl<M, P> ModelChecker<M, P>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
    P: PropMapping,
{
    pub fn new(machine: M, propmap: P, ltl: &str) -> anyhow::Result<Arc<Self>> {
        let buchi = BuchiAutomaton::from_ltl(propmap, ltl)?;
        Ok(Arc::new(Self {
            buchi,
            machine: StorePathMachine::from(machine),
        }))
    }

    pub fn initial(&self, state: M::State) -> ModelCheckerState<M::State, M::Action> {
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
#[bounded_to(M::State, M::Action, M::Error)]
pub enum ModelCheckerTransitionError<M: Machine>
where
    M::Action: Clone,
{
    BuchiError(ModelCheckerBuchiError<M>),
    MachineError(M::Error),
}

#[derive(derive_bounded::Debug)]
#[bounded_to(M::Action, M::State)]
pub struct ModelCheckerBuchiError<M: Machine>
where
    M::Action: Clone,
{
    error: BuchiError,
    path: im::Vector<M::Action>,
    states: (M::State, M::State),
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
#[bounded_to(S, A)]
pub struct ModelCheckerState<S, A>
where
    S: Clone + Debug + Eq + Hash,
    A: Clone + Debug,
{
    #[deref]
    pub pathstate: StorePathState<S, A>,
    #[debug(skip)]
    pub buchi: BuchiPaths,
}

// NB: regrettably we can't easily derive Hash because ModelChecker is not Hash,
//     even though that doesn't matter.
// TODO: make PR to derive_bounded to support Hash?
impl<S, A> Hash for ModelCheckerState<S, A>
where
    S: Clone + Debug + Eq + Hash,
    A: Clone + Debug,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pathstate.state.hash(state);
        self.buchi.hash(state);
    }
}

impl<S, A> ModelCheckerState<S, A>
where
    S: Clone + Debug + Eq + Hash,
    A: Clone + Debug,
{
    pub fn new(state: S, buchi_states: impl IntoIterator<Item = StateName>) -> Self {
        Self {
            pathstate: StorePathState::new(state),
            buchi: BuchiPaths(buchi_states.into_iter().collect()),
        }
    }

    pub fn map_state<SS>(self, f: impl FnOnce(S) -> Option<SS>) -> Option<ModelCheckerState<SS, A>>
    where
        SS: Clone + Debug + Eq + Hash,
    {
        let path = self.pathstate.path;
        let state = f(self.pathstate.state)?;
        let pathstate = StorePathState::<SS, A> { state, path };
        Some(ModelCheckerState {
            pathstate,
            buchi: self.buchi,
        })
    }
}
