//! Exhaustive state-space traversal and LTL model checking for
//! [`Machine`](polestar_core::Machine)s.
//!
//! LTL formulas are converted to Büchi automata by shelling out to the
//! external `ltl3ba` binary, which must be on the `PATH`.

#![warn(missing_docs)]

pub mod diagram;
pub mod logic;
pub mod machine_ext;
pub mod model_checker;
pub mod store_path;
pub mod traversal;

#[cfg(feature = "example-models")]
pub mod example_models;

pub use machine_ext::MachineExt;
pub use model_checker::{ModelChecker, ModelCheckerError};
pub use traversal::{Traversal, TraversalReport};

pub mod prelude;
