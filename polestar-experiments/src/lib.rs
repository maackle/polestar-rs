//! Experimental and half-baked parts of Polestar.
//!
//! Nothing here is stable. Things graduate to `polestar-core` (or a crate of
//! their own) once they are well-exercised.

#![allow(missing_docs)]

pub mod delay;
pub mod diagram;
pub mod event_handler;
pub mod generate;
pub mod mapping;
pub mod upto_lazy;

pub use event_handler::EventHandler;
pub use generate::Generator;

// Not currently compiled; kept for reference.
// pub mod unused::{actor, ltl, projection};

/// experimental
#[allow(unused)]
mod nondeterministic_automaton;
