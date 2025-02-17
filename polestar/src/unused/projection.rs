use core::fmt::Debug;
use std::sync::mpsc;

use crate::{prelude::*, util::first};

/// A Projection takes a system which may or may not be a state machine,
/// and maps it onto a finite state machine model.
/// This is useful for reaping the benefits of FSMs in systems which
/// are not or cannot be represented as state machines.
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
pub trait Projection
where
    Self::Model: Machine,
{
    type System;
    type Model;
    type Event;

    fn apply(&self, system: &mut Self::System, event: Self::Event);
    fn map_state(&mut self, system: &Self::System) -> Option<StateOf<Self::Model>>;
    fn map_event(&mut self, event: Self::Event) -> Option<ActionOf<Self::Model>>;
    fn gen_state(
        &mut self,
        generator: &mut impl Generator,
        state: StateOf<Self::Model>,
    ) -> Self::System;
    fn gen_event(
        &mut self,
        generator: &mut impl Generator,
        event: ActionOf<Self::Model>,
    ) -> Self::Event;
}

pub type StateOf<M> = <M as Machine>::State;
pub type ActionOf<M> = <M as Machine>::Action;
pub type ErrorOf<M> = <M as Machine>::Error;

pub struct ProjectionRunner<P: Projection> {
    projection: P,
    event_rx: mpsc::Receiver<P::Event>,
}

impl<P: Projection> ProjectionRunner<P> {}

#[cfg(feature = "testing")]
pub trait ProjectionTests: Sized + Projection
where
    Self::System: Clone + Debug,
    Self::Event: Clone + Debug,
    Self::Model: Machine + Clone,
    StateOf<Self::Model>: Clone + Debug + Eq,
    ActionOf<Self::Model>: Clone + Debug + Eq,
    ErrorOf<Self::Model>: Eq,
{
    fn test_commutativity(&mut self, machine: Self::Model, x: Self::System, event: Self::Event) {
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
                    machine
                        .transition(x_m.clone(), a.clone())
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
        mut self,
        runner: &mut impl Generator,
        machine: Self::Model,
        system: Self::System,
        event: Self::Event,
    ) {
        if let (Some(state), Some(transition)) =
            (self.map_state(&system), self.map_event(event.clone()))
        {
            self.map_state_is_a_retraction(runner, state.clone());
            self.map_event_is_a_retraction(runner, transition.clone());
            self.test_commutativity(machine.clone(), system, event);
            self.transition_commutes_with_generation(runner, machine, state, transition);
        }
        // TODO: all other cases ok?
    }

    fn map_state_is_a_retraction(
        &mut self,
        runner: &mut impl Generator,
        state: StateOf<Self::Model>,
    ) {
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

    fn map_event_is_a_retraction(
        &mut self,
        runner: &mut impl Generator,
        event: ActionOf<Self::Model>,
    ) {
        let e = self.gen_event(runner, event.clone());
        let roundtrip = self.map_event(e);
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
        mut self,
        runner: &mut impl Generator,
        machine: Self::Model,
        x: StateOf<Self::Model>,
        event: ActionOf<Self::Model>,
    ) {
        // if error, return original state
        let x_t = machine
            .transition(x.clone(), event.clone())
            .map(first)
            .unwrap_or_else(|_| x.clone());

        let x_tg: Self::System = self.gen_state(runner, x_t);

        let x_g = self.gen_state(runner, x);
        let mut x_gt = x_g.clone();
        let e = self.gen_event(runner, event);
        self.apply(&mut x_gt, e);

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
impl<T> ProjectionTests for T
where
    T: Projection,
    Self::System: Clone + Debug,
    Self::Event: Clone + Debug,
    T::Model: Machine + Clone,
    StateOf<T::Model>: Clone + Debug + Eq,
    ActionOf<T::Model>: Clone + Debug + Eq,
    ErrorOf<T::Model>: Eq,
{
}
