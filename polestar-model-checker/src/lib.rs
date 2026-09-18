//! LTL model checking for [`Machine`](polestar_core::Machine)s, built on
//! [`polestar_traversal`].
//!
//! LTL formulas are converted to Büchi automata by shelling out to the
//! external `ltl3ba` binary, which must be on the `PATH`.

#![warn(missing_docs)]

pub mod logic;
pub mod model_checker;
pub mod traversal_ext;

#[cfg(feature = "example-models")]
pub mod example_models;

pub use model_checker::{ModelChecker, ModelCheckerError};
pub use traversal_ext::{TraversalModelCheckExt, TraversalSpecExt};

pub mod prelude;
