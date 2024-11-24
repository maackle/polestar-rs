// TODO: this isn't really right.
// - probably need to start with the existing list of nodes, with a None status
// - there probably needs to be a special proptest strategy for pulling from the existing list of nodes
// - this model might not even be a diagrammable state machine, maybe it needs to be further abstracted into something visually comprehensible

// pub mod network_single_op;
pub mod op_family;
pub mod op_single;

pub const N: usize = 3;
pub const O: usize = 3;

pub type NodeId = polestar::id::IdU8<N>;
pub type OpId = polestar::id::IdU8<O>;
