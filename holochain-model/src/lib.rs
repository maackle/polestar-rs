#![allow(unused_imports)]

pub mod op_family;
pub mod op_family_known_deps;
pub mod op_network;
pub mod op_single;

pub const N: usize = 3;
pub const O: usize = 3;

pub type NodeId = polestar::id::IdU8<N>;
pub type OpId = polestar::id::IdU8<O>;
