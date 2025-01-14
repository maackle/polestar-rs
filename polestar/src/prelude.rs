//! Prelude for Polestar.

// pub use crate::actor::{Actor, ShareRead, ShareRw};
pub use crate::ext::MapExt;
pub use crate::generate::Generator;
pub use crate::id::*;
pub use crate::logic::{EvaluatePropositions, PropositionRegistry, Transition};
pub use crate::machine::{Machine, TransitionResult};
pub use crate::model_checker::ModelChecker;

pub use std::convert::Infallible;
