pub mod checked;
mod context;
pub mod ext;
mod refcell;

pub use context::*;
pub use refcell::*;

use std::{convert::Infallible, sync::Arc};

use proptest_derive::Arbitrary;

use crate::util::first;

pub trait Machine
where
    Self: Sized,
{
    type Action;
    type Fx;
    type Error: std::fmt::Debug;

    fn transition(self, event: Self::Action) -> MachineResult<Self>;

    /// Perform a transition and ignore the effect, when the effect is `()`.
    fn transition_(self, event: Self::Action) -> Result<Self, Self::Error>
    where
        Self: Machine<Fx = ()>,
    {
        self.transition(event).map(|(fsm, _)| fsm)
    }

    fn context<C>(self, context: C) -> Contextual<Self, C> {
        Contextual {
            fsm: self,
            context: Arc::new(context),
        }
    }

    fn checked(
        self,
        make_error: impl Fn(anyhow::Error) -> Self::Error + 'static,
    ) -> checked::Checker<Self, Self::Error> {
        checked::Checker::new(self, make_error)
    }

    fn apply_actions(
        mut self,
        actions: impl IntoIterator<Item = Self::Action>,
    ) -> Result<(Self, Vec<Self::Fx>), Self::Error> {
        let mut fxs = vec![];
        for action in actions.into_iter() {
            let (m, fx) = self.transition(action)?;
            fxs.push(fx);
            self = m;
        }
        Ok((self, fxs))
    }

    fn apply_actions_(
        self,
        actions: impl IntoIterator<Item = Self::Action>,
    ) -> Result<Self, Self::Error> {
        self.apply_actions(actions).map(first)
    }

    /// Designates this state as a terminal state.
    ///
    /// This is an optional hint, useful for generating diagrams from FSMs.
    fn is_terminal(&self) -> bool {
        false
    }
}

pub type MachineResult<S> = Result<(S, <S as Machine>::Fx), <S as Machine>::Error>;

impl Machine for bool {
    type Action = bool;
    type Fx = ();
    type Error = Infallible;

    fn transition(self, event: Self::Action) -> MachineResult<Self> {
        Ok((event, ()))
    }

    fn is_terminal(&self) -> bool {
        false
    }
}
