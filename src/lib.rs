#![feature(associated_type_defaults)]
// #![feature(lazy_type_alias)]

pub mod actor;
pub mod dfa;
pub mod generate;
pub mod id;
// pub mod lens;
pub mod event_handler;
pub mod traversal;
pub mod util;

#[cfg(feature = "testing")]
pub mod projection;

#[cfg(feature = "diagrams")]
pub mod diagram;
pub mod ext;

pub use actor::Actor;
pub use dfa::{Machine, TransitionResult};
pub use event_handler::{EventHandler, EventSink};

pub mod prelude {
    pub use crate::actor::{Actor, ShareRead, ShareRw};
    pub use crate::dfa::{Machine, TransitionResult};
    pub use crate::generate::Generator;
    pub use crate::projection::{Projection, ProjectionTests};

    pub use std::convert::Infallible;
}

/// experimental
#[allow(unused)]
mod nondeterministic_automaton;
