#![feature(associated_type_defaults)]
#![feature(lazy_type_alias)]

pub mod actor;
pub mod fsm;
pub mod generate;
// pub mod lens;
pub mod util;

#[cfg(feature = "testing")]
pub mod projection;

#[cfg(feature = "diagrams")]
pub mod diagram;

pub use fsm::Fsm;

pub mod prelude {
    pub use crate::actor::{ActorFsm, ActorRead, ActorRw};
    pub use crate::fsm::{Fsm, FsmContext, FsmHashMap, FsmRefCell, FsmResult};
    pub use crate::generate::Generator;
    pub use crate::projection::{
        ProjectionDown, ProjectionDownTests, ProjectionUp, ProjectionUpTests,
    };

    pub use std::convert::Infallible;
}
