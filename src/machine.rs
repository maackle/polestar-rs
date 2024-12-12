pub mod store_path;

// mod refcell;

// pub use refcell::*;

use crate::util::first;

pub trait Machine
where
    Self: Sized,
{
    type State;
    type Action;

    #[cfg(not(nightly))]
    type Fx;
    #[cfg(not(nightly))]
    type Error;

    #[cfg(nightly)]
    type Fx = ();
    #[cfg(nightly)]
    type Error: std::fmt::Debug = anyhow::Error;

    fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self>;

    /// Designates this state as a terminal state.
    ///
    /// This is an optional hint, useful for generating diagrams from FSMs.
    fn is_terminal(&self, _: &Self::State) -> bool;

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

    fn apply_each_action(
        &self,
        mut state: Self::State,
        actions: impl IntoIterator<Item = Self::Action>,
        on_action: impl Fn(&Self::Action, &Self::State),
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

pub type TransitionResult<S> =
    Result<(<S as Machine>::State, <S as Machine>::Fx), <S as Machine>::Error>;
