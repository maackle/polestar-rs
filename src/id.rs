use std::{collections::HashMap, hash::Hash};

use proptest::prelude::{BoxedStrategy, Strategy};
use proptest_derive::Arbitrary;

/// A number which is less than `N`.
/// Useful for IDs in exhaustive testing, to limit the number of choices.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Into, derive_more::Deref,
)]
pub struct UpTo<const N: usize>(usize);

impl<const N: usize> UpTo<N> {
    pub fn modulo(n: usize) -> Self {
        Self(n % N)
    }
}

impl<const N: usize> TryFrom<usize> for UpTo<N> {
    type Error = String;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < N {
            Ok(Self(value))
        } else {
            Err(format!("Cannot use {value} for UpTo<{N}>"))
        }
    }
}

impl<const N: usize> exhaustive::Exhaustive for UpTo<N> {
    fn generate(u: &mut exhaustive::DataSourceTaker) -> exhaustive::Result<Self> {
        u.choice(N).map(Self)
    }
}

impl<const N: usize> proptest::arbitrary::Arbitrary for UpTo<N> {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        (0..N).prop_map(Self).boxed()
    }
}

pub trait Id:
    Clone
    + Copy
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Hash
    + std::fmt::Display
    + std::fmt::Debug
    + proptest::arbitrary::Arbitrary
    + exhaustive::Exhaustive
{
}

impl<const N: usize> Id for IdU8<N> {}

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Arbitrary,
    derive_more::Into,
    derive_more::Deref,
)]
pub struct IdU8<const N: usize>(usize);

impl<const N: usize> IdU8<N> {
    pub fn new(n: usize) -> Self {
        Self::try_from(n).expect("Attempted to initialize Id<{N}> with {n}")
    }

    pub fn modulo(n: usize) -> Self {
        Self(n % N)
    }
}

impl<const N: usize> exhaustive::Exhaustive for IdU8<N> {
    fn generate(u: &mut exhaustive::DataSourceTaker) -> exhaustive::Result<Self> {
        u.choice(N).map(Self)
    }
}

impl<const N: usize> TryFrom<usize> for IdU8<N> {
    type Error = String;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < N {
            Ok(Self(value))
        } else {
            Err(format!("Cannot use {value} for Id<{N}>"))
        }
    }
}

impl<const N: usize> std::fmt::Debug for IdU8<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

impl<const N: usize> std::fmt::Display for IdU8<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct IdMap<const N: usize, V> {
    map: HashMap<V, IdU8<N>>,
}

impl<V, const N: usize> Default for IdMap<N, V>
where
    V: Hash + Eq,
{
    fn default() -> Self {
        Self {
            map: HashMap::default(),
        }
    }
}

impl<V, const N: usize> IdMap<N, V>
where
    V: Hash + Eq,
{
    pub fn lookup(&mut self, k: V) -> Result<IdU8<N>, String> {
        let len = self.map.len();
        match self.map.entry(k) {
            std::collections::hash_map::Entry::Occupied(e) => Ok(e.get().clone()),
            std::collections::hash_map::Entry::Vacant(e) => {
                let id = IdU8::try_from(len)?;
                e.insert(id);
                Ok(id)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_id_map() {
        let mut m = IdMap::<3, _>::default();
        assert_eq!(m.lookup("c"), Ok(IdU8(0)));
        assert_eq!(m.lookup("m"), Ok(IdU8(1)));
        assert_eq!(m.lookup("c"), Ok(IdU8(0)));
        assert_eq!(m.lookup("y"), Ok(IdU8(2)));
        assert_eq!(m.lookup("y"), Ok(IdU8(2)));
        assert!(matches!(m.lookup("k"), Err(_)));
    }
}
