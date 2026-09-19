//! The core of Polestar: the [`Machine`] trait and the small set of
//! well-exercised helpers around it (bounded ids, discrete time, map helpers).
//!
//! Everything to do with state-space traversal and model checking lives in
//! `polestar-model-checker`; experimental material lives in `polestar-experiments`.

#![warn(missing_docs)]
#![cfg_attr(nightly, feature(associated_type_defaults))]

pub mod ext;
pub mod id;
pub mod machine;
pub mod time;
pub mod util;

pub use machine::{
    ActionOf, Behavior, BehaviorModel, ErrorOf, FxOf, Machine, MachineUnit, StateMachine,
    StateModel, StateOf, TransitionResult,
};

pub mod prelude;
