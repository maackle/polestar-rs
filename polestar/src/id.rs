use std::{collections::HashMap, hash::Hash};

mod upto;
mod upto_lazy;
pub use upto::*;
pub use upto_lazy::*;

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
    + Send
    + Sync
    + 'static
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
            std::collections::hash_map::Entry::Occupied(e) => Ok(*e.get()),
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
    fn test_arithmetic() {
        let a = UpTo::<3, true>::wrapping(1);
        let b = UpTo::<3, true>::wrapping(2);
        assert_eq!(a + 1, UpTo(2));
        assert_eq!(b - 1, UpTo(1));
        assert_eq!(a + 3, UpTo(1));
        assert_eq!(b - 3, UpTo(2));
        assert_eq!(a + 4, UpTo(2));
        assert_eq!(b - 4, UpTo(1));

        assert_eq!(a + (3 * 100 + 1), UpTo(2));
        assert_eq!(b - (3 * 100 + 1), UpTo(1));
    }

    #[test]
    fn test_id_map() {
        let mut m = IdMap::<_, UpTo<3>>::default();
        assert_eq!(m.lookup("c"), Ok(UpTo(0)));
        assert_eq!(m.lookup("m"), Ok(UpTo(1)));
        assert_eq!(m.lookup("c"), Ok(UpTo(0)));
        assert_eq!(m.lookup("y"), Ok(UpTo(2)));
        assert_eq!(m.lookup("y"), Ok(UpTo(2)));
        assert!(m.lookup("k").is_err());
    }
}
