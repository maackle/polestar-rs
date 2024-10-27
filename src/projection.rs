use core::fmt::Debug;

use crate::prelude::*;
use proptest::prelude::*;

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

    fn apply(self, event: Self::Event) -> Self;

    fn map_event(&self, event: Self::Event) -> M::Event;
    fn map_state(&self) -> M;

    fn gen_event(&self, generator: &mut impl Generator, event: M::Event) -> Self::Event;
    fn gen_state(&self, generator: &mut impl Generator, state: M) -> Self;
}

// #[cfg(feature = "testing")]
pub trait ProjectionTests<M>: Sized + Projection<M>
where
    Self: Clone + Debug,
    Self::Event: Clone + Debug,
    M: Fsm + Clone + Debug + Eq + Arbitrary,
    M::Event: Clone + Debug + Eq + Arbitrary,
{
    fn test_invariants(self, runner: &mut impl Generator, event: Self::Event) {
        let state = self.map_state();
        let transition = self.map_event(event.clone());
        self.map_state_is_a_retraction(runner, state.clone());
        self.map_event_is_a_retraction(runner, transition.clone());
        self.clone().transition_commutes_with_mapping(event);
        self.transition_commutes_with_generation(runner, state, transition);
    }

    fn map_state_is_a_retraction(&self, runner: &mut impl Generator, state: M) {
        let generated = self.gen_state(runner, state.clone());
        let roundtrip: M = Self::map_state(&generated);
        assert_eq!(
            state, roundtrip,
            "map_state_is_a_retraction failed:    state != map_state(gen_state(_, state))"
        )
    }

    fn map_event_is_a_retraction(&self, runner: &mut impl Generator, event: M::Event) {
        let roundtrip: M::Event = self.map_event(self.gen_event(runner, event.clone()));
        assert_eq!(
            event, roundtrip,
            "map_event_is_a_retraction failed:   transition != map_event(gen_event(_, transition))"
        )
    }

    fn transition_commutes_with_mapping(self, event: Self::Event) {
        let left: M = Self::map_state(&self.clone().apply(event.clone()));
        let mut right = self.map_state();
        let _ = M::transition(&mut right, self.map_event(event));
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
        let left: Self = {
            let mut state = state.clone();
            let _ = state.transition(event.clone());
            self.gen_state(runner, state)
        };
        let right: Self = Self::apply(self.gen_state(runner, state), self.gen_event(runner, event));
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
{
}
