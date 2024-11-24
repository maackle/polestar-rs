pub mod checked;

// mod refcell;

// pub use refcell::*;

use crate::util::first;

pub trait Machine
where
    Self: Sized,
{
    type State;
    type Action;
    type Fx;
    type Error: std::fmt::Debug;

    fn transition(&self, state: Self::State, event: Self::Action) -> MachineResult<Self>;

    /// Perform a transition and ignore the effect, when the effect is `()`.
    fn transition_(
        &self,
        state: Self::State,
        event: Self::Action,
    ) -> Result<Self::State, Self::Error>
    where
        Self: Machine<Fx = ()>,
    {
        self.transition(state, event).map(first)
    }

    fn checked(self) -> checked::Checker<Self> {
        checked::Checker::new(self)
    }

    fn apply_actions(
        &self,
        mut state: Self::State,
        actions: impl IntoIterator<Item = Self::Action>,
    ) -> Result<(Self::State, Vec<Self::Fx>), Self::Error> {
        let mut fxs = vec![];
        for action in actions.into_iter() {
            let (s, fx) = self.transition(state, action)?;
            fxs.push(fx);
            state = s;
        }
        Ok((state, fxs))
    }

    fn apply_actions_(
        &self,
        state: Self::State,
        actions: impl IntoIterator<Item = Self::Action>,
    ) -> Result<Self::State, Self::Error> {
        self.apply_actions(state, actions).map(first)
    }

    /// Designates this state as a terminal state.
    ///
    /// This is an optional hint, useful for generating diagrams from FSMs.
    fn is_terminal(&self, _: &Self::State) -> bool;
}

pub type MachineResult<S> =
    Result<(<S as Machine>::State, <S as Machine>::Fx), <S as Machine>::Error>;
