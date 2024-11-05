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

    // fn transition_mut(&mut self, event: Self::Event) -> Result<Self::Fx, Self::Error>
    // where
    //     Self: Default,
    // {
    //     let state = std::mem::take(self).transition(event)?
    //     self.transition_mut(event).map(|(fsm, _)| fsm)
    // }

    fn context<C>(self, context: C) -> Contextual<Self, C> {
        Contextual {
            fsm: self,
            context: Arc::new(context),
        }
    }
}

pub type FsmResult<S: Fsm> = Result<(S, S::Fx), S::Error>;

// pub trait FsmMut {
//     type Event;
//     type Fx;
//     type Error: std::fmt::Debug;

//     fn transition_mut(&mut self, event: Self::Event) -> Self::Fx;
// }

// impl<S> Fsm for Result<S, S::Error>
// where
//     S: FsmMut,
// {
//     type Event = S::Event;
//     type Fx = S::Fx;
//     type Error = S::Error;

//     fn transition(mut self, event: Self::Event) -> FsmResult<Self> {
//         let fx = FsmMut::transition_mut(&mut self, event);
//         Ok((self, FsmMut::transition_mut(&mut self, event)))
//     }
// }

/// Wrapper around an FSM which carries a context that gets injected into each event.
/// Useful for attaching some immutable context to the FSM which is not part of its own state.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Arbitrary)]
pub struct Contextual<S: Fsm, C> {
    fsm: S,
    // TODO: C: Clone
    context: Arc<C>,
}

impl<S, C> std::fmt::Debug for Contextual<S, C>
where
    S: Fsm + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.fsm)
    }
}

impl<S, C> Contextual<S, C>
where
    S: Fsm,
{
    pub fn new(fsm: S, context: C) -> Self {
        Self {
            fsm,
            context: Arc::new(context),
        }
    }
}

impl<S, C, E> Fsm for Contextual<S, C>
where
    S: Fsm<Event = (E, Arc<C>)>,
{
    type Event = E;
    type Fx = S::Fx;
    type Error = S::Error;

    fn transition(self, event: Self::Event) -> FsmResult<Self> {
        let context = self.context;
        let (fsm, fx) = Fsm::transition(self.fsm, (event, context.clone()))?;
        Ok((Self { fsm, context }, fx))
    }
}

impl Fsm for bool {
    type Event = bool;
    type Fx = ();
    type Error = Infallible;

    fn transition(self, event: Self::Event) -> FsmResult<Self> {
        Ok((event, ()))
    }
}
