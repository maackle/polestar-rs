#![feature(associated_type_defaults)]

pub mod actor;
pub mod fsm;
pub mod generate;
pub mod projection;

pub use fsm::Fsm;

pub mod prelude {
    pub use crate::actor::{ActorRead, ActorRw};
    pub use crate::fsm::Fsm;
    pub use crate::generate::Generate;
    pub use crate::projection::{Projection, ProjectionTests};
}
