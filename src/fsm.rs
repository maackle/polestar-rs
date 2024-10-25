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
    fn context<C>(self, context: C) -> Contextual<Self, C>;
}

impl<T> FsmExt for T
where
    T: Fsm,
{
    fn context<C>(self, context: C) -> Contextual<Self, C> {
        Contextual {
            fsm: self,
            context: Arc::new(context),
        }
    }
}

pub trait FsmPure {
    type Event;

    fn transition(&mut self, event: Self::Event);
}

impl<S> FsmPure for S
where
    S: Fsm<Fx = ()>,
{
    type Event = S::Event;

    fn transition(&mut self, event: Self::Event) {
        let () = Fsm::transition(self, event);
    }
}

/// Wrapper around an FSM which carries a context that gets injected into each event.
/// Useful for attaching some immutable context to the FSM which is not part of its own state.
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
