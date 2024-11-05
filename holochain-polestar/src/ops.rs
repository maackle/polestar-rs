use std::sync::Arc;

use nanoid::nanoid;
use proptest::prelude::{BoxedStrategy, Strategy};
use proptest_derive::Arbitrary;

static UNIQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(Arc<str>);

impl Id {
    pub fn new() -> Self {
        Self::from_int(UNIQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }

    pub fn from_int(mut x: u32) -> Self {
        let mut s = String::new();
        while x > 0 {
            s.push(std::char::from_digit((x % 26) + 10 - 1, 36).unwrap());
            x /= 26;
        }
        Self(s.to_ascii_uppercase().into())
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl proptest::prelude::Arbitrary for Id {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        let hi = UNIQ.load(std::sync::atomic::Ordering::Relaxed).max(2);
        (1..hi).prop_map(Self::from_int).boxed()
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
    derive_more::Display,
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
    derive_more::Display,
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
    derive_more::Display,
)]
pub struct OpHash(Id);

impl From<&Op> for OpHash {
    fn from(op: &Op) -> Self {
        op.hash.clone()
    }
}

mod model;
mod system;
