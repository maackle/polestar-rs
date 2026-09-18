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
//! re-exported here wholesale. The other crates are re-exported behind features:
//! `traversal` (`polestar-traversal`), `diagram` (`polestar-diagram`) and
//! `model-checker` (`polestar-model-checker`). Experimental material lives in
//! `polestar-experiments` and is not re-exported.

#![warn(missing_docs)]

pub use polestar_core::*;

#[cfg(feature = "traversal")]
pub use polestar_traversal::{MachineExt, Traversal, TraversalReport, store_path, traversal};

#[cfg(feature = "diagram")]
pub use polestar_diagram as diagram;

#[cfg(feature = "model-checker")]
pub use polestar_model_checker::{
    ModelChecker, ModelCheckerError, TraversalModelCheckExt, TraversalSpecExt, logic, model_checker,
};

/// Commonly used items from `polestar-core`, plus those from the optional
/// crates whose features are enabled.
pub mod prelude {
    pub use polestar_core::prelude::*;

    #[cfg(feature = "traversal")]
    pub use polestar_traversal::prelude::*;

    #[cfg(feature = "model-checker")]
    pub use polestar_model_checker::prelude::*;
}
