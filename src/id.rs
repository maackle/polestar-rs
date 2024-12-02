use std::{collections::HashMap, hash::Hash};

use proptest::prelude::{BoxedStrategy, Strategy};
use proptest_derive::Arbitrary;

pub trait Id:
    Clone
    + Copy
    + Default
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Hash
    + TryFrom<usize>
    + std::fmt::Display
    + std::fmt::Debug
{
    fn choices() -> IdChoices {
        IdChoices::Large
    }
}

impl Id for u8 {}
impl Id for u16 {}
impl Id for u32 {}
impl Id for u64 {}
impl Id for usize {}

pub enum IdChoices {
    Small(usize),
    Large,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Arbitrary,
    exhaustive::Exhaustive,
)]
pub struct IdUnit;

impl Id for IdUnit {
    fn choices() -> IdChoices {
        IdChoices::Small(1)
    }
}

impl std::fmt::Display for IdUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "∅")
    }
}

impl TryFrom<usize> for IdUnit {
    type Error = String;
    fn try_from(x: usize) -> Result<Self, Self::Error> {
        if x == 0 {
            Ok(IdUnit)
        } else {
            Err(format!("Cannot use {x} for IdUnit"))
        }
    }
}

/// A number in the range [0, N)
#[derive(
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::Into,
    derive_more::Deref,
)]
pub struct UpTo<const N: usize>(usize);

impl<const N: usize> Id for UpTo<N> {
    fn choices() -> IdChoices {
        IdChoices::Small(N)
    }
}

impl<const N: usize> UpTo<N> {
    pub fn new(n: usize) -> Self {
        Self::try_from(n).expect("Attempted to initialize Id<{N}> with {n}")
    }

    pub fn modulo(n: usize) -> Self {
        Self(n % N)
    }

    pub fn all_values() -> [Self; N] {
        (0..N)
            .map(Self::new)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

impl<const N: usize> exhaustive::Exhaustive for UpTo<N> {
    fn generate(u: &mut exhaustive::DataSourceTaker) -> exhaustive::Result<Self> {
        u.choice(N).map(Self)
    }
}

impl<const N: usize> TryFrom<usize> for UpTo<N> {
    type Error = String;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < N {
            Ok(Self(value))
        } else {
            Err(format!("Cannot use {value} for Id<{N}>"))
        }
    }
}

impl<const N: usize> std::fmt::Debug for UpTo<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

impl<const N: usize> std::fmt::Display for UpTo<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct IdMap<V, I: Id> {
    map: HashMap<V, I>,
}

impl<V, I> Default for IdMap<V, I>
where
    I: Id,
    V: Hash + Eq,
{
    fn default() -> Self {
        Self {
            map: HashMap::default(),
        }
    }
}

impl<V, I> IdMap<V, I>
where
    I: Id,
    I::Error: std::fmt::Debug,
    V: Hash + Eq,
{
    pub fn lookup(&mut self, v: V) -> Result<I, String> {
        let len = self.map.len();
        match self.map.entry(v) {
            std::collections::hash_map::Entry::Occupied(e) => Ok(e.get().clone()),
            std::collections::hash_map::Entry::Vacant(e) => {
                let id = I::try_from(len).map_err(|e| format!("{e:?}"))?;
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
        let mut m = IdMap::<_, UpTo<3>>::default();
        assert_eq!(m.lookup("c"), Ok(UpTo(0)));
        assert_eq!(m.lookup("m"), Ok(UpTo(1)));
        assert_eq!(m.lookup("c"), Ok(UpTo(0)));
        assert_eq!(m.lookup("y"), Ok(UpTo(2)));
        assert_eq!(m.lookup("y"), Ok(UpTo(2)));
        assert!(matches!(m.lookup("k"), Err(_)));
    }
}
