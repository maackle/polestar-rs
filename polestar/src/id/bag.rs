use std::{collections::BTreeSet, ops::ShrAssign};

use exhaustive::Exhaustive;

/// An unsorted set of values with a maximum size of N.
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::Into,
    derive_more::Deref,
    derive_more::Display,
)]
#[display("{:?}", _0)]
#[cfg_attr(feature = "recording", derive(serde::Serialize, serde::Deserialize))]
pub struct Bag<const N: usize, T>(BTreeSet<T>)
where
    T: Ord + TryFrom<usize>;

impl<const N: usize, T: Ord + TryFrom<usize>> Bag<N, T> {
    /// Constructor
    pub fn new(values: impl IntoIterator<Item = T>) -> Self {
        Self(values.into_iter().collect())
    }

    /// Intersect with another bag
    pub fn extend(&mut self, values: impl IntoIterator<Item = T>) {
        self.0.extend(values);
    }
}

impl<const N: usize, T: Ord + TryFrom<usize>> From<Vec<T>> for Bag<N, T> {
    fn from(vec: Vec<T>) -> Self {
        Self(vec.into_iter().collect())
    }
}

impl<const N: usize, T> Exhaustive for Bag<N, T>
where
    T: Ord + TryFrom<usize>,
    T::Error: std::fmt::Debug,
{
    fn generate(u: &mut exhaustive::DataSourceTaker) -> exhaustive::Result<Self> {
        let mut set = BTreeSet::new();

        let mut c = u.choice(2usize.pow(N.try_into().expect("N is too large")))?;
        let mut i = 0;
        while c > 0 {
            let add = (c & 1) == 1;
            if add {
                match T::try_from(i) {
                    Ok(t) => {
                        set.insert(t);
                    }
                    Err(e) => {
                        debug_assert!(false, "Error converting {c}: {e:?}");
                        tracing::error!("Error converting {c}: {e:?}");
                        return Err(exhaustive::ChoiceError);
                    }
                }
            }
            c.shr_assign(1);
            i += 1;
        }
        Ok(Self(set))
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::id::UpTo;

    use super::*;

    #[test]
    fn test_bag() {
        type T = UpTo<3>;
        type B = Bag<3, T>;
        let bags = B::iter_exhaustive(None).collect_vec();
        let bags: BTreeSet<B> = bags.into_iter().collect();

        let expected = [
            vec![],
            vec![T::new(0)],
            vec![T::new(1)],
            vec![T::new(2)],
            vec![T::new(0), T::new(1)],
            vec![T::new(0), T::new(2)],
            vec![T::new(1), T::new(2)],
            vec![T::new(0), T::new(1), T::new(2)],
        ]
        .into_iter()
        .map(B::from)
        .collect();

        pretty_assertions::assert_eq!(bags, expected);
        println!("{:?}", bags);
    }
}
