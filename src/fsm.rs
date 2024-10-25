use std::sync::Arc;

pub trait Fsm
where
    Self: Sized,
{
    type Event;
    type Fx = ();

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
        let _ = Fsm::transition(self, event);
    }
}

/// Wrapper around an FSM which carries a context that gets injected into each event.
/// Useful for attaching some immutable context to the FSM which is not part of its own state.
pub struct Contextual<F: Fsm, C> {
    fsm: F,
    context: Arc<C>,
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

// pub trait Pfsm<'a> {
//     type Event;
//     type State: Fsm<Event = (Self::Event, &'a Self::Meta)>;
//     type Meta: 'a;
//     type Fx;

//     fn meta(&self) -> &Self::Meta;
//     fn state_mut(&mut self) -> &mut Self::State;

//     fn transition(&mut self, event: Self::Event) -> Self::Fx {
//         Self::State::transition(self.state_mut(), (event, self.meta()))
//     }
// }

// // impl<S, T> Fsm for T
// // where
// //     S: Fsm,
// //     T: Pfsm<State = S> + Sized,
// // {
// //     type Event = T::Event;
// //     type Fx = T::Fx;

// //     fn transition(self, transition: Self::Event) -> (Self, Self::Fx) {
// //         Fsm::transition(self, transition)(self, ())
// //     }
// // }
