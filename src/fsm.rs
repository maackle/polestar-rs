mod btreemap;
mod context;
mod hashmap;
mod refcell;

pub use btreemap::*;
pub use context::*;
pub use hashmap::*;
pub use refcell::*;

use std::{convert::Infallible, sync::Arc};

use proptest_derive::Arbitrary;

pub trait Fsm
where
    Self: Sized,
{
    type Action;
    type Fx;
    type Error: std::fmt::Debug;

    fn transition(self, event: Self::Action) -> FsmResult<Self>;

    /// Perform a transition and ignore the effect, when the effect is `()`.
    fn transition_(self, event: Self::Action) -> Result<Self, Self::Error>
    where
        Self: Fsm<Fx = ()>,
    {
        self.transition(event).map(|(fsm, _)| fsm)
    }

    fn context<C>(self, context: C) -> FsmContext<Self, C> {
        FsmContext {
            fsm: self,
            context: Arc::new(context),
        }
    }

    /// Designates this state as a terminal state.
    ///
    /// This is an optional hint, useful for generating diagrams from FSMs.
    fn is_terminal(&self) -> bool {
        false
    }
}

pub type FsmResult<S: Fsm> = Result<(S, S::Fx), S::Error>;

impl Fsm for bool {
    type Action = bool;
    type Fx = ();
    type Error = Infallible;

    fn transition(self, event: Self::Action) -> FsmResult<Self> {
        Ok((event, ()))
    }

    fn is_terminal(&self) -> bool {
        false
    }
}
