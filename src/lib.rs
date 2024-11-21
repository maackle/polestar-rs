// #![feature(associated_type_defaults)]
// #![feature(lazy_type_alias)]

pub mod actor;
pub mod dfa;
pub mod generate;
pub mod id;
pub mod nfm;
// pub mod lens;
pub mod util;

#[cfg(feature = "testing")]
pub mod projection;

#[cfg(feature = "diagrams")]
pub mod diagram;

pub use actor::Actor;
pub use dfa::{Machine, MachineResult};

pub mod prelude {
    pub use crate::actor::{Actor, ShareRead, ShareRw};
    pub use crate::dfa::{ext::*, Contextual, FsmRefCell, Machine, MachineResult};
    pub use crate::generate::Generator;
    pub use crate::projection::{Projection, ProjectionTests};

    pub use std::convert::Infallible;
}
