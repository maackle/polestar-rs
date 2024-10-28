use core::{marker::PhantomData, ops::Deref};
use std::sync::Arc;

pub trait Fsm
where
    Self: Sized,
{
    type Event;
    type Fx;

    fn transition(&mut self, event: Self::Event) -> Self::Fx;
}

pub trait FsmExt: Fsm {
    fn context<C>(self, context: C) -> Contextual<Self, C> {
        Contextual {
            fsm: self,
            context: Arc::new(context),
        }
    }

    fn map<T: Fsm>(
        self,
        state: impl FnOnce(Self) -> T,
        event: impl FnOnce(Self::Event) -> T::Event,
        fx: impl FnOnce(Self::Fx) -> T::Fx,
    ) -> Mapped<Self, T, _, _, _, _> {
        Mapped {
            inner: self,
            state_map: state,
            event_map: event,
            fx_map: fx,
            target: PhantomData,
            phantom: PhantomData,
        }
    }
}

impl<T> FsmExt for T where T: Fsm {}

struct Mapped<S, E, F, FS, FE, FF> {
    inner: S,
    state_map: FS,
    event_map: FE,
    fx_map: FF,
    target: PhantomData<E>,
    phantom: PhantomData<F>,
}

impl<S, E, F, FS, FE, FF> Fsm for Mapped<S, E, F, FS, FE, FF>
where
    S: Fsm,
    E: Deref<Target = S::Event>,
    FS: FnOnce(S) -> Self,
    FE: FnOnce(S::Event) -> E,
    FF: FnOnce(S::Fx) -> F,
{
    type Event = E;
    type Fx = F;

    fn transition(&mut self, event: Self::Event) -> Self::Fx {
        self.fx_map(S::transition(&mut self.inner, *event))
    }
}

// pub trait FsmPure {
//     type Event;

//     fn transition(&mut self, event: Self::Event);
// }

// impl<S> FsmPure for S
// where
//     S: Fsm<Fx = ()>,
// {
//     type Event = S::Event;

//     fn transition(&mut self, event: Self::Event) {
//         let () = Fsm::transition(self, event);
//     }
// }

/// Wrapper around an FSM which carries a context that gets injected into each event.
/// Useful for attaching some immutable context to the FSM which is not part of its own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contextual<F: Fsm, C> {
    fsm: F,
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

/// Convenience for updating state by returning an optional owned value
pub fn maybe_update<S, E>(s: &mut S, f: impl FnOnce(&S) -> (Option<S>, E)) -> E
where
    S: Sized,
{
    let (next, fx) = f(s);
    if let Some(next) = next {
        *s = next;
    }
    fx
}

/// Convenience for updating state by returning an owned value
pub fn update_replace<S, E>(s: &mut S, f: impl FnOnce(&S) -> (S, E)) -> E
where
    S: Sized + Clone,
{
    let (next, fx) = f(s);
    *s = next;
    fx
}

/// Convenience for updating state by returning an owned value
pub fn update_copy<S, E>(s: &mut S, f: impl FnOnce(S) -> (S, E)) -> E
where
    S: Sized + Copy,
{
    let (next, fx) = f(*s);
    *s = next;
    fx
}
