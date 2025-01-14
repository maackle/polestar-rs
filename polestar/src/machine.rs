//! Traits defining state machines, the foundation of polestar Models.

pub mod store_path;

use std::{fmt::Debug, hash::Hash};

use exhaustive::Exhaustive;

use crate::{traversal::Traversal, util::first};

/// A type alias for all the trait bounds required for a
/// State or Action of a Machine
/// (a machine is made out of cogs, get it?)
pub trait Cog: Clone + Debug + Eq + Hash + Send + Sync {}
impl<T: Clone + Debug + Eq + Hash + Send + Sync> Cog for T {}

/// The essential trait which defines a state machine, specifically
/// a deterministic finite automaton.
pub trait Machine
where
    Self: Sized + Send + Sync + 'static,
{
    /// The type representing the states of the machine
    type State: Cog;
    /// The type representing the actions (transitions) of the machine
    type Action: Cog;

    /// The type representing the side effects of the machine
    #[cfg(not(nightly))]
    type Fx;

    /// The type corresponding to invalid state transitions
    #[cfg(not(nightly))]
    type Error: Debug + Send + Sync;

    #[cfg(nightly)]
    type Fx = ();
    #[cfg(nightly)]
    type Error: Debug = anyhow::Error;

    /// Defines the transition function of the machine.
    fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self>;

    /// Designates this state as a terminal state.
    ///
    /// This is an optional hint, useful for generating diagrams from FSMs.
    fn is_terminal(&self, _: &Self::State) -> bool {
        false
    }

    /// Perform a transition and ignore the effect, when the effect is `()`.
    fn transition_(
        &self,
        state: Self::State,
        action: Self::Action,
    ) -> Result<Self::State, Self::Error>
    where
        Self: Machine<Fx = ()>,
    {
        self.transition(state, action).map(first)
    }

    /// Create a new [`Traversal`] for this machine.
    fn traverse(self, initial: impl IntoIterator<Item = Self::State>) -> Traversal<Self>
    where
        Self::State: Clone + Debug,
        Self::Action: Clone + Debug + Exhaustive,
    {
        Traversal::new(self, initial)
    }

    /// Apply a sequence of actions to a state, causing a transition for each one,
    /// with a callback function called after each transition.
    fn apply_each_action(
        &self,
        mut state: Self::State,
        actions: impl IntoIterator<Item = Self::Action>,
        mut on_action: impl FnMut(&Self::Action, &Self::State),
    ) -> Result<(Self::State, Vec<Self::Fx>), (Self::Error, Self::State, Self::Action)>
    where
        Self::State: Clone,
        Self::Action: Clone,
    {
        let mut fxs = vec![];
        for action in actions.into_iter() {
            let (s, fx) = self
                .transition(state.clone(), action.clone())
                .map_err(|e| (e, state, action.clone()))?;
            on_action(&action, &s);
            fxs.push(fx);
            state = s;
        }
        Ok((state, fxs))
    }

    /// Apply a sequence of actions to a state, causing a transition for each one,
    /// collecting the list of effects and returning the final state.
    fn apply_actions(
        &self,
        state: Self::State,
        actions: impl IntoIterator<Item = Self::Action>,
    ) -> Result<(Self::State, Vec<Self::Fx>), (Self::Error, Self::State, Self::Action)>
    where
        Self::State: Clone,
        Self::Action: Clone,
    {
        self.apply_each_action(state, actions, |_, _| ())
    }

    /// Apply actions but throw away the effects.
    fn apply_actions_(
        &self,
        state: Self::State,
        actions: impl IntoIterator<Item = Self::Action>,
    ) -> Result<Self::State, (Self::Error, Self::State, Self::Action)>
    where
        Self::State: Clone,
        Self::Action: Clone,
    {
        self.apply_actions(state, actions).map(first)
    }
}

/// Helper type for the return value of the [`Machine::transition`] function.
pub type TransitionResult<S> =
    Result<(<S as Machine>::State, <S as Machine>::Fx), <S as Machine>::Error>;

impl Machine for () {
    type State = ();
    type Action = ();
    type Fx = ();
    type Error = ();

    fn transition(&self, (): (), (): ()) -> TransitionResult<Self> {
        Ok(((), ()))
    }
}
