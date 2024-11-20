use std::{collections::HashMap, hash::Hash};

use proptest::prelude::{BoxedStrategy, Strategy};

/// A number which is less than `N`.
/// Useful for IDs in exhaustive testing, to limit the number of choices.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Into, derive_more::Deref,
)]
pub struct UpTo<const N: usize>(usize);

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
            Err(format!("Cannot use {value} for UpTo<{N}>"))
        }
    }
}

impl<const N: usize> proptest::arbitrary::Arbitrary for UpTo<N> {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        (0..N).prop_map(Self).boxed()
    }
}

#[derive(Debug, Default)]
pub struct IdMap<const N: usize, V> {
    map: HashMap<V, UpTo<N>>,
}

impl<V, const N: usize> IdMap<N, V>
where
    V: Hash + Eq,
{
    pub fn insert(&mut self, k: V) -> Result<UpTo<N>, String> {
        let len = self.map.len();
        match self.map.entry(k) {
            std::collections::hash_map::Entry::Occupied(e) => Ok(e.get().clone()),
            std::collections::hash_map::Entry::Vacant(e) => {
                let id = UpTo::try_from(len)?;
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
        assert_eq!(m.insert("c"), Ok(UpTo(0)));
        assert_eq!(m.insert("m"), Ok(UpTo(1)));
        assert_eq!(m.insert("c"), Ok(UpTo(0)));
        assert_eq!(m.insert("y"), Ok(UpTo(2)));
        assert_eq!(m.insert("y"), Ok(UpTo(2)));
        assert!(matches!(m.insert("k"), Err(_)));
    }
}
