#![cfg_attr(nightly, feature(associated_type_defaults))]

pub mod actor;
pub mod generate;
pub mod id;
pub mod machine;
pub mod model_checker;
// pub mod lens;
pub mod event_handler;
pub mod ext;
pub mod logic;
pub mod ltl;
pub mod mapping;
pub mod time;
pub mod traversal;
pub mod util;

// pub mod projection;

#[cfg(feature = "diagrams")]
pub mod diagram;

#[cfg(feature = "example-models")]
pub mod example_models;

pub use actor::Actor;
pub use event_handler::{EventHandler, EventSink};
pub use machine::{Machine, TransitionResult};

pub mod prelude {
    pub use crate::actor::{Actor, ShareRead, ShareRw};
    pub use crate::ext::MapExt;
    pub use crate::generate::Generator;
    pub use crate::id::*;
    pub use crate::machine::{Machine, TransitionResult};
    // pub use crate::projection::{Projection, ProjectionTests};

    pub use std::convert::Infallible;
}

/// experimental
#[allow(unused)]
mod nondeterministic_automaton;
