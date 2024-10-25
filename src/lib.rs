use prop::{strategy::ValueTree, test_runner::TestRunner};
use proptest::{arbitrary::Arbitrary, prelude::*};
use std::fmt::Debug;

mod fsm;
use fsm::Fsm;

/// Invariants:
///
///     map_state(gen_state(_, state)) == state
///     map_event(gen_event(_, transition)) == transition
///     map_state(apply(x, event)) == transition(map_state(x), map_event(event))
///     map_state(gen_state(_, transition(state, transition))) == map_state(apply(gen_state(_, state), gen_event(_, transition)))
///
pub trait Projection<M>
where
    M: Fsm + Arbitrary,
    M::Transition: Arbitrary,
{
    type Event;

    fn apply(self, event: Self::Event) -> Self;

    fn map_event(&self, event: Self::Event) -> M::Transition;
    fn map_state(&self) -> M;

    fn gen_event(&self, runner: &mut TestRunner, transition: M::Transition) -> Self::Event;
    fn gen_state(&self, runner: &mut TestRunner, state: M) -> Self;
}

// #[cfg(feature = "testing")]
pub trait ProjectionTests<M>: Sized + Projection<M>
where
    Self: Clone + Debug,
    Self::Event: Clone + Debug,
    M: Fsm + Clone + Debug + Eq + Arbitrary,
    M::Transition: Clone + Debug + Eq + Arbitrary,
{
    fn test_invariants(self, runner: &mut TestRunner, event: Self::Event) {
        let state = self.map_state();
        let transition = self.map_event(event.clone());
        self.map_state_is_a_retraction(runner, state.clone());
        self.map_event_is_a_retraction(runner, transition.clone());
        self.clone().transition_commutes_with_mapping(event);
        self.transition_commutes_with_generation(runner, state, transition);
    }

    fn map_state_is_a_retraction(&self, runner: &mut TestRunner, state: M) {
        let generated = self.gen_state(runner, state.clone());
        let roundtrip: M = Self::map_state(&generated);
        assert_eq!(
            state, roundtrip,
            "map_state_is_a_retraction failed:    state != map_state(gen_state(_, state))"
        )
    }

    fn map_event_is_a_retraction(&self, runner: &mut TestRunner, transition: M::Transition) {
        let roundtrip: M::Transition = self.map_event(self.gen_event(runner, transition.clone()));
        assert_eq!(
            transition, roundtrip,
            "map_event_is_a_retraction failed:   transition != map_event(gen_event(_, transition))"
        )
    }

    fn transition_commutes_with_mapping(self, event: Self::Event) {
        let left: M = Self::map_state(&self.clone().apply(event.clone()));
        let right: M = M::transition(self.map_state(), self.map_event(event));
        assert_eq!(
            left, right,
            "transition_commutes_with_mapping failed:    map_state(apply(x, event)) != transition(map_state(x), map_event(event))"
        )
    }

    fn transition_commutes_with_generation(
        self,
        runner: &mut TestRunner,
        state: M,
        transition: M::Transition,
    ) {
        let left: Self = self.gen_state(runner, M::transition(state.clone(), transition.clone()));
        let right: Self = Self::apply(
            self.gen_state(runner, state),
            self.gen_event(runner, transition),
        );
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
    M::Transition: Clone + Debug + Eq + Arbitrary,
{
}

pub trait ArbitraryExt {
    fn generate_arbitrary<T: Arbitrary>(&mut self) -> Result<T, prop::test_runner::Reason> {
        self.generate_with(T::arbitrary())
    }

    fn generate_with<T: Arbitrary>(
        &mut self,
        strategy: T::Strategy,
    ) -> Result<T, prop::test_runner::Reason>;
}

impl ArbitraryExt for TestRunner {
    fn generate_with<T: Arbitrary>(
        &mut self,
        strategy: T::Strategy,
    ) -> Result<T, prop::test_runner::Reason> {
        Ok(strategy.new_tree(self)?.current())
    }
}

// pub trait Generator {
//     fn generate<T: Arbitrary>(&mut self) -> T;
// }

// impl Generator for TestRunner {
//     fn generate<T: Arbitrary>(&mut self) -> T {
//         T::arbitrary().new_tree(self).unwrap().current()
//     }
// }
