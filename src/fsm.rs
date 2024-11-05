mod context;
mod hashmap;
mod refcell;

pub use context::*;
pub use hashmap::*;
pub use refcell::*;

use std::{convert::Infallible, sync::Arc};

use proptest_derive::Arbitrary;

pub trait Fsm
where
    Self: Sized,
{
    type Event;
    type Fx;
    type Error: std::fmt::Debug;

    fn transition(self, event: Self::Event) -> FsmResult<Self>;

    fn transition_(self, event: Self::Event) -> Result<Self, Self::Error>
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
}

pub type FsmResult<S: Fsm> = Result<(S, S::Fx), S::Error>;

impl Fsm for bool {
    type Event = bool;
    type Fx = ();
    type Error = Infallible;

    fn transition(self, event: Self::Event) -> FsmResult<Self> {
        Ok((event, ()))
    }
}
