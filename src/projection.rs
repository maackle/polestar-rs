use core::fmt::Debug;

use crate::{prelude::*, util::first};
use proptest::prelude::*;

/// A Projection takes a system which may or may not an FSM, and maps it onto
/// an FSM. This is useful for reaping the benefits of FSMs in systems which
/// are not or cannot be represented as FSMs.
///
/// Invariants:
///
/// - map_state(gen_state(_, state)) == state
/// - map_event(gen_event(_, transition)) == transition
/// - map_state(apply(x, event)) == transition(map_state(x), map_event(event))
/// - map_state(gen_state(_, transition(state, transition))) == map_state(apply(gen_state(_, state), gen_event(_, transition)))
///
pub trait Projection<M>
where
    M: Fsm + Arbitrary,
    M::Event: Arbitrary,
{
    type Event;

    fn apply(&mut self, event: Self::Event);

    fn map_state(&self) -> Option<M>;
    fn map_event(&self, event: Self::Event) -> Option<M::Event>;

    fn gen_state(&self, generator: &mut impl Generator, state: M) -> Self;
    fn gen_event(&self, generator: &mut impl Generator, event: M::Event) -> Self::Event;
}

// #[cfg(feature = "testing")]
pub trait ProjectionTests<M>: Sized + Projection<M>
where
    Self: Clone + Debug,
    Self::Event: Clone + Debug,
    M: Fsm + Clone + Debug + Eq + Arbitrary,
    M::Event: Clone + Debug + Eq + Arbitrary,
    M::Error: Eq,
{
    fn test_invariants(self, runner: &mut impl Generator, event: Self::Event) {
        if let (Some(state), Some(transition)) = (self.map_state(), self.map_event(event.clone())) {
            self.map_state_is_a_retraction(runner, state.clone());
            self.map_event_is_a_retraction(runner, transition.clone());
            self.clone().transition_commutes_with_mapping(event);
            self.transition_commutes_with_generation(runner, state, transition);
        }
        // TODO: all other cases ok?
    }

    fn map_state_is_a_retraction(&self, runner: &mut impl Generator, state: M) {
        let generated = self.gen_state(runner, state.clone());
        let roundtrip = Self::map_state(&generated);
        assert_eq!(
            Some(state),
            roundtrip,
            "map_state_is_a_retraction failed:    state != map_state(gen_state(_, state))"
        )
    }

    fn map_event_is_a_retraction(&self, runner: &mut impl Generator, event: M::Event) {
        let roundtrip = self.map_event(self.gen_event(runner, event.clone()));
        assert_eq!(
            Some(event),
            roundtrip,
            "map_event_is_a_retraction failed:   transition != map_event(gen_event(_, transition))"
        )
    }

    fn transition_commutes_with_mapping(self, event: Self::Event) {
        let left = {
            let mut state = self.clone();
            state.apply(event.clone());
            state.map_state().map(Ok)
        };
        let right = {
            let s = self.map_state();
            let e = self.map_event(event);
            if let (Some(s), Some(e)) = (s, e) {
                Some(M::transition(s, e).map(first))
            } else {
                None
            }
        };
        assert_eq!(
            left, right,
            "transition_commutes_with_mapping failed:    map_state(apply(x, event)) != transition(map_state(x), map_event(event))"
        )
    }

    fn transition_commutes_with_generation(
        self,
        runner: &mut impl Generator,
        state: M,
        event: M::Event,
    ) {
        let left: Self =
            { self.gen_state(runner, state.clone().transition(event.clone()).unwrap().0) };
        let mut right = self.gen_state(runner, state);
        right.apply(self.gen_event(runner, event));
        assert_eq!(left.map_state(), right.map_state(), "transition_commutes_with_generation failed:    map_state(gen_state(_, transition(state, transition))) != map_state(apply(gen_state(_, state), gen_event(_, transition)))")
    }
}

// #[cfg(feature = "testing")]
impl<M, T> ProjectionTests<M> for T
where
    T: Projection<M>,
    Self: Clone + Debug,
    Self::Event: Clone + Debug,
    M: Fsm + Clone + Debug + Eq + Arbitrary,
    M::Event: Clone + Debug + Eq + Arbitrary,
    M::Error: Eq,
{
}
