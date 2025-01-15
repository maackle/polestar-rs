//! Performs model checking by connecting a state machine with a Buchi automaton
//! which represents a set of safety and liveness specifications.

pub mod buchi;

#[cfg(test)]
mod tests;

use std::{fmt::Debug, hash::Hash};

use buchi::*;

use crate::logic::{EvaluatePropositions, PropositionMapping, Transition};
use crate::machine::{
    store_path::{StorePathMachine, StorePathState},
    Machine, TransitionResult,
};

/// A model checker which connects a state machine with a Buchi automaton
/// to check a set of safety and liveness specifications.
pub struct ModelChecker<M, P>
where
    M: Machine,
    P: PropositionMapping,
{
    buchi: BuchiAutomaton<M, P>,
    machine: StorePathMachine<M>,
}

/// Model checkers can fail due to either safety or liveness violations.
#[derive(derive_bounded::Debug)]
#[bounded_to(M::State, M::Action)]
pub enum ModelCheckerError<M: Machine>
where
    M::Action: Clone,
{
    /// A safety violation occurred, meaning that something that wasn't supposed to happen did.
    Safety {
        /// The sequence of actions that led to the bad state.
        path: im::Vector<M::Action>,
        /// The last two states that were checked (one good, one bad).
        states: (M::State, M::State),
    },
    /// A liveness violation occurred, meaning that something that was supposed to happen never did.
    Liveness {
        /// All paths that lead to the loop which causes the liveness violation..
        paths: Vec<im::Vector<M::Action>>,
    },
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
    P: PropositionMapping + Send + Sync + 'static,
    // TODO: if a proc macro is ever written, make it clearer that you must implement Propositions for
    // pairs + action, not just the state. (or somehow make this easier)
    Transition<M>: EvaluatePropositions<P::Proposition>,
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
            .transition(state.clone(), action.clone())
            .map_err(ModelCheckerTransitionError::MachineError)?;

        let buchi_next = self
            .buchi
            .transition_(buchi, Transition(prev.clone(), action, next.state.clone()))
            .map_err(|error| {
                ModelCheckerTransitionError::BuchiError(ModelCheckerBuchiError {
                    error,
                    path: next.path.clone(),
                    states: (prev, next.state.clone()),
                })
            })?;

        let next = ModelCheckerState {
            pathstate: next,
            buchi: buchi_next,
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
    P: PropositionMapping,
{
    /// Create a model checker from a state machine, a proposition name mapping,
    /// and an LTL formula.
    pub fn from_ltl(machine: M, propmap: P, ltl: &str) -> anyhow::Result<Self> {
        let buchi = BuchiAutomaton::from_ltl(propmap, ltl)?;
        Ok(Self {
            buchi,
            machine: StorePathMachine::from(machine),
        })
    }

    /// Given a model's state, return an initial state for the model checker which
    /// corresponds with the model's initial state.
    pub fn initial(&self, state: M::State) -> ModelCheckerState<M::State, M::Action> {
        let inits = self
            .buchi
            .states
            .keys()
            .filter(|&name| name.ends_with("_init"))
            .cloned();

        ModelCheckerState::new(state, inits)
    }
}

/// Specifies whether an error occured in the model, or in the Buchi automaton.
#[derive(derive_bounded::Debug)]
#[bounded_to(M::State, M::Action, M::Error)]
pub enum ModelCheckerTransitionError<M: Machine>
where
    M::Action: Clone,
{
    /// An error occured in the Buchi automaton, meaning the specification is not satisfied.
    BuchiError(ModelCheckerBuchiError<M>),
    /// An error occured in the model, meaning that an invalid action was taken.
    MachineError(M::Error),
}

/// Information about a Buchi automaton error.
#[derive(derive_bounded::Debug)]
#[bounded_to(M::Action, M::State)]
pub struct ModelCheckerBuchiError<M: Machine>
where
    M::Action: Clone,
{
    /// The error that occured in the Buchi automaton.
    pub error: BuchiError,
    /// The path that led to the error.
    pub path: im::Vector<M::Action>,
    /// The last two states that were checked (one good, one bad).
    pub states: (M::State, M::State),
}

/*        █████               █████
         ░░███               ░░███
  █████  ███████    ██████   ███████    ██████
 ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
░░█████   ░███      ███████   ░███    ░███████
 ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
 ██████   ░░█████ ░░████████  ░░█████ ░░██████
░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░  */

/// The State used in the [`ModelChecker`]
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
    /// The state of the model, including the path taken to get there.
    #[deref]
    pub pathstate: StorePathState<S, A>,

    /// The set of Buchi states that need to be checked and transitioned.
    ///
    /// Remember, the Buchi automaton is nondeterministic, so we need to check
    /// all possible paths, of which there may be many at once.
    #[debug(skip)]
    pub(crate) buchi: BuchiStateNames,
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
    /// Constructor
    pub fn new(state: S, buchi_states: impl IntoIterator<Item = StateName>) -> Self {
        Self {
            pathstate: StorePathState::new(state),
            buchi: BuchiStateNames(buchi_states.into_iter().collect()),
        }
    }

    /// Map the model state to a new type while keeping the Buchi state the same.
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
