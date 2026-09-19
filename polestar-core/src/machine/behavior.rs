//! Behaviors: drivers that choose a Machine's actions.
//!
//! A [`Machine`] defines what *can* happen; a [`Behavior`] decides what
//! *does* happen. Different behaviors drive the same model in different
//! ways: happy paths, known failure modes, chaos, seeded randomness.
//! Where exhaustive traversal enumerates every enabled action, a behavior
//! picks a particular sequence of them, which is what a simulation is.
//!
//! The driver loop alternates two calls:
//! - [`Behavior::next_tick`] reads the model state and proposes actions;
//! - [`Behavior::handle_fx`] consumes each transition's effects, typically
//!   translating them into actions to propose on a *later* tick (e.g. a
//!   `Send` effect becomes a `Recv` action after a modeled delay).
//!
//! Effects a behavior does not consume bubble up to whatever is driving
//! it, so behaviors nest the same way effectful machines do.
//!
//! # Errors
//!
//! In a [`Machine`], `Err` means "action not enabled". A behavior is
//! expected to propose only enabled actions: when a proposed action is
//! refused by the model, [`BehaviorModel`] treats that as a bug in the
//! behavior and fails the transition rather than skipping the action.
//! Errors returned by the behavior itself are likewise fatal.
//!
//! # Determinism
//!
//! A behavior may hold mutable state (pending queues, a seeded RNG), and
//! in [`BehaviorModel`] that state rides inside the machine's `State`.
//! Keep *all* nondeterminism there — an RNG must be seeded and stored in
//! the behavior, never ambient — and the composite remains a genuinely
//! deterministic machine: replay from the initial state is exact.

use crate::machine::{ActionOf, FxOf, Machine, StateOf, TransitionResult};

/// Logic which drives a model by proposing actions and consuming effects.
///
/// See the [module docs](self) for the driver contract.
pub trait Behavior {
    /// The machine being driven.
    type Model: Machine<Error = anyhow::Error>;

    /// Produce the actions to apply next, given the current model state.
    /// This generally corresponds to a single "tick" of a simulation.
    fn next_tick(
        &mut self,
        state: &StateOf<Self::Model>,
    ) -> anyhow::Result<Vec<ActionOf<Self::Model>>>;

    /// Consume the effects of one transition, updating behavior state so
    /// that future [`Behavior::next_tick`] calls take them into account.
    ///
    /// `state` is the model state immediately *after* the transition that
    /// produced `fx` — not the state at the end of the tick — so effect
    /// translation (e.g. expanding a broadcast against current topology)
    /// sees the world as it was when the effect happened.
    ///
    /// Return effects you do not handle, so they bubble up a level; the
    /// default handles nothing.
    fn handle_fx(
        &mut self,
        _state: &StateOf<Self::Model>,
        fx: FxOf<Self::Model>,
    ) -> anyhow::Result<Option<FxOf<Self::Model>>> {
        Ok(Some(fx))
    }
}

impl<B: Behavior + ?Sized> Behavior for Box<B> {
    type Model = B::Model;

    fn next_tick(
        &mut self,
        state: &StateOf<Self::Model>,
    ) -> anyhow::Result<Vec<ActionOf<Self::Model>>> {
        self.as_mut().next_tick(state)
    }

    fn handle_fx(
        &mut self,
        state: &StateOf<Self::Model>,
        fx: FxOf<Self::Model>,
    ) -> anyhow::Result<Option<FxOf<Self::Model>>> {
        self.as_mut().handle_fx(state, fx)
    }
}

/// A model of some behavior: a [`Behavior`] closed over its [`Machine`],
/// forming a machine with no choices left.
///
/// Each `()` action runs one tick: the behavior proposes actions, each is
/// applied in turn, and each transition's effects are handed to the
/// behavior *before the next action runs*. Unhandled effects are the
/// composite's own `Fx`, one entry per inner transition.
///
/// This is a closed system — the action carries no information, so from a
/// given state exactly one run unfolds. It is a `Machine` so that it
/// composes like any other (it can be stepped, nested, driven by an outer
/// behavior), but traversing it is pointless: the branching factor is 1,
/// and behavior state (queues, RNG positions) would distinguish states
/// that are equal for every purpose a checker cares about. Traverse the
/// underlying model; run this one.
#[derive(Clone, Debug)]
pub struct BehaviorModel<B: Behavior> {
    model: B::Model,
}

impl<B: Behavior> BehaviorModel<B> {
    /// Constructor
    pub fn new(model: B::Model) -> Self {
        Self { model }
    }

    /// The underlying model.
    pub fn model(&self) -> &B::Model {
        &self.model
    }
}

impl<B> Machine for BehaviorModel<B>
where
    B: Behavior + Clone + std::fmt::Debug + Send + Sync + 'static,
{
    type State = (B, StateOf<B::Model>);
    type Action = ();
    type Fx = Vec<FxOf<B::Model>>;
    type Error = anyhow::Error;

    fn transition(
        &self,
        (mut behavior, mut state): Self::State,
        (): Self::Action,
    ) -> TransitionResult<Self> {
        let mut unhandled = vec![];
        for action in behavior.next_tick(&state)? {
            let (next, fx) = self.model.transition(state, action)?;
            state = next;
            if let Some(fx) = behavior.handle_fx(&state, fx)? {
                unhandled.push(fx);
            }
        }
        Ok(((behavior, state), unhandled))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A counter that emits its new value as an effect.
    #[derive(Clone, Debug)]
    struct Counter;

    impl Machine for Counter {
        type State = u32;
        type Action = u32;
        type Fx = u32;
        type Error = anyhow::Error;

        fn transition(&self, state: u32, add: u32) -> TransitionResult<Self> {
            anyhow::ensure!(add > 0, "zero add not enabled");
            Ok((state + add, state + add))
        }
    }

    /// Adds 1 and 2 each tick, and records the state each effect was
    /// observed at.
    #[derive(Clone, Debug, Default)]
    struct AddTwice {
        seen: Vec<(u32, u32)>, // (state at handle_fx, fx)
    }

    impl Behavior for AddTwice {
        type Model = Counter;

        fn next_tick(&mut self, _state: &u32) -> anyhow::Result<Vec<u32>> {
            Ok(vec![1, 2])
        }

        fn handle_fx(&mut self, state: &u32, fx: u32) -> anyhow::Result<Option<u32>> {
            self.seen.push((*state, fx));
            // Handle odd effects, bubble even ones.
            Ok(fx.is_multiple_of(2).then_some(fx))
        }
    }

    #[test]
    fn effects_are_handled_at_per_action_state() {
        let bm = BehaviorModel::<AddTwice>::new(Counter);
        let ((behavior, state), unhandled) = bm.transition((AddTwice::default(), 0), ()).unwrap();
        assert_eq!(state, 3);
        // Each effect saw the state its own transition produced,
        // not the end-of-tick state.
        assert_eq!(behavior.seen, vec![(1, 1), (3, 3)]);
        // Both effects are odd, so both were handled; nothing bubbles.
        assert_eq!(unhandled, Vec::<u32>::new());

        let ((behavior, state), unhandled) = bm.transition((behavior, state), ()).unwrap();
        assert_eq!(state, 6);
        assert_eq!(unhandled, vec![4, 6], "even effects bubble up");
        assert_eq!(behavior.seen.len(), 4);
    }

    #[test]
    fn a_disabled_action_fails_the_tick() {
        #[derive(Clone, Debug)]
        struct ProposesDisabled;
        impl Behavior for ProposesDisabled {
            type Model = Counter;
            fn next_tick(&mut self, _: &u32) -> anyhow::Result<Vec<u32>> {
                Ok(vec![0])
            }
        }
        let bm = BehaviorModel::<ProposesDisabled>::new(Counter);
        assert!(bm.transition((ProposesDisabled, 0), ()).is_err());
    }
}
