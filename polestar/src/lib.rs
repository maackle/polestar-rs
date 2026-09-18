//! Polestar is a flexible, hackable pattern and toolkit for
//! modeling and formal verification of concurrent/distributed systems.
//!
//! It is a spiritual kin of other modeling tools like
//! [TLA+](https://lamport.azurewebsites.net/tla/tla.html?from=https://research.microsoft.com/en-us/um/people/lamport/tla/tla.html&type=path)
//! and the [P](https://p-org.github.io/P/) language. However, while these
//! are designed as complete environments with their own in-built specification
//! language, Polestar is implemented as a set of tools and patterns that can
//! be mixed, matched, and modified to fit your formal verification needs.
//!
//! This crate is a facade. The stable core lives in `polestar-core` and is
//! re-exported here wholesale. Model checking lives in `polestar-model-checker`
//! and is re-exported under the `model-checker` feature. Experimental material
//! lives in `polestar-experiments` and is not re-exported.

#![warn(missing_docs)]

pub use polestar_core::*;

#[cfg(feature = "model-checker")]
pub use polestar_model_checker::{
    MachineExt, ModelChecker, ModelCheckerError, Traversal, TraversalReport, diagram, logic,
    model_checker, store_path, traversal,
};

/// Commonly used items from `polestar-core`, plus those from
/// `polestar-model-checker` when the `model-checker` feature is enabled.
pub mod prelude {
    pub use polestar_core::prelude::*;

    #[cfg(feature = "model-checker")]
    pub use polestar_model_checker::prelude::*;
}
