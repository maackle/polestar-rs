//! Polestar is a flexible, hackable pattern and toolkit for
//! modeling and formal verification of concurrent/distributed systems.
//!
//! It is a spiritual kin of other modeling tools like
//! [TLA+](https://lamport.azurewebsites.net/tla/tla.html?from=https://research.microsoft.com/en-us/um/people/lamport/tla/tla.html&type=path)
//! and the [P](https://p-org.github.io/P/) language. However, while these
//! are designed as complete environments with their own in-built specification
//! language, Polestar is implemented as a set of tools and patterns that can
//! be mixed, matched, and modified to fit your formal verification needs.

#![warn(missing_docs)]
#![cfg_attr(nightly, feature(associated_type_defaults))]

pub mod event_handler;
pub mod ext;
pub mod generate;
pub mod id;
pub mod logic;
pub mod machine;
pub mod mapping;
pub mod model_checker;
pub mod time;
pub mod traversal;
pub mod util;

// pub mod ltl;
// pub mod actor;
// pub mod projection;

#[cfg(feature = "diagrams")]
pub mod diagram;

#[cfg(feature = "example-models")]
pub mod example_models;

pub use event_handler::EventHandler;
pub use machine::{Machine, TransitionResult};

pub mod prelude;

/// experimental
#[allow(unused)]
mod nondeterministic_automaton;
