//! Breadth-first traversal of a [`Machine`](polestar_core::Machine)'s state graph.
//!
//! A [`Traversal`] is the starting point for graphing a state space
//! (`polestar-diagram`), searching for terminal states, and model checking
//! (`polestar-model-checker`).

#![warn(missing_docs)]

pub mod machine_ext;
pub mod store_path;
pub mod traversal;

pub use machine_ext::MachineExt;
pub use traversal::{TerminalSet, Traversal, TraversalReport, VisitType};

pub mod prelude;
