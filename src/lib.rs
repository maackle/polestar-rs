#![feature(associated_type_defaults)]
#![feature(lazy_type_alias)]

pub mod actor;
pub mod fsm;
pub mod generate;
// pub mod lens;
pub mod fsm_wrappers;
pub mod projection;
pub mod util;

#[cfg(feature = "diagrams")]
pub mod diagram;

pub use fsm::Fsm;

pub mod prelude {
    pub use crate::actor::{ActorFsm, ActorRead, ActorRw};
    pub use crate::fsm::{Contextual, Fsm, FsmResult};
    pub use crate::fsm_wrappers::FsmCell;
    pub use crate::generate::Generator;
    pub use crate::projection::{Projection, ProjectionTests};

    pub use std::convert::Infallible;
}
