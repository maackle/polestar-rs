mod projection_result;

use core::fmt::Debug;

use crate::{prelude::*, util::first};

/// A Projection takes a system which may or may not an FSM, and maps it onto
/// an FSM. This is useful for reaping the benefits of FSMs in systems which
/// are not or cannot be represented as FSMs.
///
/// Invariants:
///
/// commutativity for down projection:
/// - map_state(apply(x, event)) == transition(map_state(x), map_event(event))
///
/// retractions:
/// - map_state(gen_state(_, state)) == state
/// - map_event(gen_event(_, transition)) == transition
///
/// commutativity for up projection:
/// - transition(state, transition) == map_state(apply(gen_state(_, state), gen_event(_, transition)))
///
///
pub trait Projection<Model>
where
    Model: Fsm,
{
    type System;
    type Event;

    fn apply(&self, system: &mut Self::System, event: Self::Event);
    fn map_state(&self, system: &Self::System) -> Option<Model>;
    fn map_event(&self, event: Self::Event) -> Option<Model::Action>;
    fn gen_state(&self, generator: &mut impl Generator, state: Model) -> Self::System;
    fn gen_event(&self, generator: &mut impl Generator, event: Model::Action) -> Self::Event;
}

#[cfg(feature = "testing")]
pub trait ProjectionTests<Model>: Sized + Projection<Model>
where
    Self::System: Clone + Debug,
    Self::Event: Clone + Debug,
    Model: Fsm + Clone + Debug + Eq,
    Model::Action: Clone + Debug + Eq,
    Model::Error: Eq,
{
    fn test_commutativity(&self, x: Self::System, event: Self::Event) {
        let x_m = self.map_state(&x);

        let x_a = {
            let mut x = x.clone();
            self.apply(&mut x, event.clone());
            x
        };

        let x_am = self.map_state(&x_a);

        let a = self.map_event(event.clone());

        let x_ma = {
            if let (Some(x_m), Some(a)) = (&x_m, &a) {
                // if error, return original state
                Some(
                    x_m.clone()
                        .transition(a.clone())
                        .map(first)
                        .unwrap_or(x_m.clone()),
                )
            } else {
                None
            }
        };

        assert_eq!(
            x_am,
            x_ma,
            "
`transition_commutes_with_mapping` failed.

original system event:
{event:#?}

system transition diff : (previous system state) vs (new system state):
{}

model transition diff : (previous model state) vs (new model state):
{}

commutative diff : (system-transitioned and mapped) vs (mapped and model-transitioned): 
{}
",
            prettydiff::diff_lines(&format!("{:#?}", x), &format!("{:#?}", x_a)),
            prettydiff::diff_lines(&format!("{:#?}", x_m), &format!("{:#?}", x_ma)),
            prettydiff::diff_lines(&format!("{:#?}", x_am), &format!("{:#?}", x_ma)),
        )
    }

    fn test_all_invariants(
        self,
        runner: &mut impl Generator,
        system: Self::System,
        event: Self::Event,
    ) {
        if let (Some(state), Some(transition)) =
            (self.map_state(&system), self.map_event(event.clone()))
        {
            self.map_state_is_a_retraction(runner, state.clone());
            self.map_event_is_a_retraction(runner, transition.clone());
            self.test_commutativity(system, event);
            self.transition_commutes_with_generation(runner, state, transition);
        }
        // TODO: all other cases ok?
    }

    fn map_state_is_a_retraction(&self, runner: &mut impl Generator, state: Model) {
        let generated = self.gen_state(runner, state.clone());
        let roundtrip = self.map_state(&generated);
        assert_eq!(
            Some(&state),
            roundtrip.as_ref(),
            "`map_state_is_a_retraction` failed:\n{}",
            prettydiff::diff_lines(
                &format!("{:#?}", Some(&state)),
                &format!("{:#?}", roundtrip)
            )
        )
    }

    fn map_event_is_a_retraction(&self, runner: &mut impl Generator, event: Model::Action) {
        let roundtrip = self.map_event(self.gen_event(runner, event.clone()));
        assert_eq!(
            Some(&event),
            roundtrip.as_ref(),
            "`map_event_is_a_retraction` failed:\n{}",
            prettydiff::diff_lines(
                &format!("{:#?}", Some(&event)),
                &format!("{:#?}", roundtrip.as_ref())
            )
        )
    }

    fn transition_commutes_with_generation(
        self,
        runner: &mut impl Generator,
        x: Model,
        event: Model::Action,
    ) {
        // if error, return original state
        let x_t = x
            .clone()
            .transition(event.clone())
            .map(first)
            .unwrap_or_else(|_| x.clone());

        let x_tg: Self::System = self.gen_state(runner, x_t);

        let x_g = self.gen_state(runner, x);
        let mut x_gt = x_g.clone();
        self.apply(&mut x_gt, self.gen_event(runner, event));

        let x_tgm = self.map_state(&x_tg);
        let x_gtm = self.map_state(&x_gt);
        assert_eq!(
            x_tgm,
            x_gtm,
            "`transition_commutes_with_generation` failed:\n{}",
            prettydiff::diff_lines(&format!("{:#?}", x_tgm), &format!("{:#?}", x_gtm))
        )
    }
}

#[cfg(feature = "testing")]
impl<M, T> ProjectionTests<M> for T
where
    T: Projection<M>,
    Self::System: Clone + Debug,
    Self::Event: Clone + Debug,
    M: Fsm + Clone + Debug + Eq,
    M::Action: Clone + Debug + Eq,
    M::Error: Eq,
{
}
