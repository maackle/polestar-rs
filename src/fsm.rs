use std::{marker::PhantomData, sync::Arc};

use proptest_derive::Arbitrary;

pub trait Fsm
where
    Self: Sized,
{
    type Event;
    type Fx;

    fn transition(&mut self, event: Self::Event) -> Self::Fx;

    fn context<C>(self, context: C) -> Contextual<Self, C> {
        Contextual {
            fsm: self,
            context: Arc::new(context),
        }
    }
}

// pub trait FsmExt: Fsm {
//     fn context<C>(self, context: C) -> Contextual<Self, C>;
// }

// impl<T> FsmExt for T where T: Fsm {}

/// Wrapper around an FSM which carries a context that gets injected into each event.
/// Useful for attaching some immutable context to the FSM which is not part of its own state.
#[derive(Debug, Clone, PartialEq, Eq, Arbitrary)]
pub struct Contextual<F: Fsm, C> {
    fsm: F,
    // TODO: C: Clone
    context: Arc<C>,
}

impl<F, C> Contextual<F, C>
where
    F: Fsm,
{
    pub fn new(fsm: F, context: C) -> Self {
        Self {
            fsm,
            context: Arc::new(context),
        }
    }
}

impl<F, C, E> Fsm for Contextual<F, C>
where
    F: Fsm<Event = (E, Arc<C>)>,
{
    type Event = E;
    type Fx = F::Fx;

    fn transition(&mut self, event: Self::Event) -> Self::Fx {
        Fsm::transition(&mut self.fsm, (event, Arc::clone(&self.context)))
    }
}

impl Fsm for bool {
    type Event = bool;
    type Fx = ();

    fn transition(&mut self, event: Self::Event) -> Self::Fx {
        *self = event;
    }
}
