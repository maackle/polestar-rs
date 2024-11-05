use std::sync::Arc;

use nanoid::nanoid;
use proptest::prelude::{BoxedStrategy, Strategy};
use proptest_derive::Arbitrary;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(Arc<str>);

impl Id {
    pub fn new() -> Self {
        Self(nanoid!(5).into())
    }
}

impl proptest::prelude::Arbitrary for Id {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        proptest::string::string_regex("[a-zA-Z0-9]{5}")
            .unwrap()
            .prop_map(|s| Self(s.into()))
            .boxed()
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Arbitrary,
    derive_more::From,
    derive_more::Deref,
)]
pub struct NodeId(Id);

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Arbitrary,
    derive_more::From,
    derive_more::Deref,
)]
pub struct Agent(Id);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Arbitrary, derive_more::From)]
pub struct Op {
    pub hash: OpHash,
    pub deps: Vec<OpHash>,
}

impl Op {
    pub fn new(hash: OpHash, deps: Vec<OpHash>) -> Self {
        Self { hash, deps }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Arbitrary, derive_more::From)]
pub struct OpHash(Id);

impl From<&Op> for OpHash {
    fn from(op: &Op) -> Self {
        op.hash.clone()
    }
}

mod model;
mod system;
