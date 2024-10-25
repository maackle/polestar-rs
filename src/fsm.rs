pub trait Fsm {
    type Transition;

    fn transition(self, transition: Self::Transition) -> Self;
}
