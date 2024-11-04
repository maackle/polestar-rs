#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(String);

impl Id {
    pub fn new() -> Self {
        Self(nanoid::nanoid!())
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

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From, derive_more::Deref,
)]
pub struct Op(Id);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From)]
pub struct OpHash(String);

impl From<&Op> for OpHash {
    fn from(op: &Op) -> Self {
        OpHash(op.0 .0.clone())
    }
}

mod model;
mod system;
