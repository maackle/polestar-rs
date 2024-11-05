use core::{marker::PhantomData, ops::Deref};
use std::sync::Arc;

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

pub trait FsmFx
where
    Self: Sized,
{
    type Event;
    type Fx;

    fn transition(self, event: Self::Event) -> (Self, Self::Fx);

    // fn context<C>(self, context: C) -> Contextual<Self, C> {
    //     Contextual {
    //         fsm: self,
    //         context: Arc::new(context),
    //     }
    // }
}

impl<F> FsmFx for F
where
    F: Fsm,
{
    type Event = F::Event;
    type Fx = F::Fx;

    fn transition(mut self, event: Self::Event) -> (Self, Self::Fx) {
        let fx = Fsm::transition(&mut self, event);
        (self, fx)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Arbitrary, derive_more::From)]
pub struct FsmO<F>(F);

impl<F> FsmO<F>
where
    F: FsmFx<Fx = ()>,
{
    pub fn transition(self, event: F::Event) -> Self {
        F::transition(self.0, event).0.into()
    }
}

/// Wrapper around an FSM which carries a context that gets injected into each event.
/// Useful for attaching some immutable context to the FSM which is not part of its own state.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Arbitrary)]
pub struct Contextual<F: Fsm, C> {
    fsm: F,
    // TODO: C: Clone
    context: Arc<C>,
}

impl<F, C> std::fmt::Debug for Contextual<F, C>
where
    F: Fsm + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.fsm)
    }
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

// impl FsmFx for bool {
//     type Event = bool;
//     type Fx = ();

//     fn transition(self, event: Self::Event) -> (Self, Self::Fx) {
//         (event, ())
//     }
// }
