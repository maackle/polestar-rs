pub trait Fsm
where
    Self: Sized,
{
    type Transition;
    type Fx = ();

    fn transition(self, transition: Self::Transition) -> (Self, Self::Fx);
}

pub trait FsmPure {
    type Transition;

    fn transition(self, transition: Self::Transition) -> Self;
}

impl<S> Fsm for S
where
    S: FsmPure,
{
    type Transition = S::Transition;
    type Fx = ();

    fn transition(self, transition: Self::Transition) -> (Self, ()) {
        (FsmPure::transition(self, transition), ())
    }
}

pub trait Pfsm {
    type Event;
    type Meta;
    type Fx;

    fn meta(&self) -> &Self::Meta;
}

impl<T> Fsm for T
where
    T: Pfsm + Sized,
{
    type Transition = Self::Transition;
    type Fx = Self::Fx;

    fn transition(self, transition: Self::Transition) -> (Self, Self::Fx) {
        (self, ())
    }
}
