// TODO: this isn't really right.
// - probably need to start with the existing list of nodes, with a None status
// - there probably needs to be a special proptest strategy for pulling from the existing list of nodes
// - this model might not even be a diagrammable state machine, maybe it needs to be further abstracted into something visually comprehensible

pub mod network_single_op;
pub mod single_op;

pub const N: usize = 3;
pub const O: usize = 3;

pub type NodeId = polestar::id::Id<N>;
pub type OpId = polestar::id::Id<O>;
