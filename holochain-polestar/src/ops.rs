use nanoid::nanoid;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(String);

impl Id {
    pub fn new() -> Self {
        Self(nanoid!(5))
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From, derive_more::Deref,
)]
pub struct NodeId(Id);

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From, derive_more::Deref,
)]
pub struct Agent(Id);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From)]
pub struct Op {
    pub hash: OpHash,
    pub deps: Vec<OpHash>,
}

impl Op {
    pub fn new(hash: OpHash, deps: Vec<OpHash>) -> Self {
        Self { hash, deps }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From)]
pub struct OpHash(Id);

impl From<&Op> for OpHash {
    fn from(op: &Op) -> Self {
        op.hash.clone()
    }
}

mod model;
mod system;
