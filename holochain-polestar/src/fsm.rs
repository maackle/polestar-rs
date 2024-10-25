use std::marker::PhantomData;

pub struct ParamFsm<State, Event, Meta, Fx> {
    state: State,
    event: Event,
    meta: Meta,
    fx: PhantomData<Fx>,
}

pub trait Pfsm
where
    Self: Sized,
{
    type Event;
    type Meta;
    type Fx;

    fn meta(&self) -> &Self::Meta;
}

impl<T> polestar::Fsm for T
where
    T: Pfsm + Sized,
{
    type Transition = Self::Transition;
    type Fx = Self::Fx;

    fn transition(self, transition: Self::Transition) -> (Self, Self::Fx) {
        (self, ())
    }
}

// impl<S, E, M, Fx> polestar::Fsm for ParamFsm<S, E, M, Fx> {
//     type Transition = E;
//     type Fx = Fx;

//     fn transition(self, transition: Self::Transition) -> (Self, Self::Fx) {
//         (self, ())
//     }
// }
