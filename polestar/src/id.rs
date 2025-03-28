//! Types related to identifiers in models.

use std::{collections::HashMap, hash::Hash};

mod upto;
pub use upto::*;

#[cfg(feature = "nonessential")]
mod upto_lazy;
#[cfg(feature = "nonessential")]
pub use upto_lazy::*;

use proptest_derive::Arbitrary;

/// A type that is suitable for use as an identifier in models.
///
/// The use of identifiers in models is a crucial consideration.
/// The space of possible ID values determines the space of distinct
/// items in a model. For a model to be exhaustively explorable,
/// this space must be small, and so using a type like [`UpTo`]
/// is a good choice. However, for a model to map onto a real-world
/// system, the number of ID values must match the number of IDs in
/// the real system.
///
/// By defining the ID types of a model generically, the same model
/// can be used in both contexts.
///
///
/// ```
/// use polestar::prelude::*;
/// use std::collections::BTreeMap;
///
/// type Node = ();
///
/// /// State is generic over ID
/// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// struct State<N: Id> {
///     nodes: BTreeMap<N, Node>,
/// }
///
/// /// Action is also generic over ID
/// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// enum Action<N: Id> {
///     Message { to: N, content: String },
/// }
///
/// struct Model<N: Id>(std::marker::PhantomData<N>);
///
/// /// Define the machine for models with IDs limited to [0, 1, 2]
/// impl Machine for Model<UpTo<3>> {
///     type State = State<UpTo<3>>;
///     type Action = Action<UpTo<3>>;
///     type Error = ();
///     type Fx = ();
///
///     fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self> {
///         todo!()
///     }
/// }
/// ```
pub trait Id:
    Clone
    + Copy
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
    /// Specifies the number of possible values for this type.
    /// (See [`IdChoices`] for more details.)
    fn choices() -> IdChoices {
        IdChoices::Large
    }
}

impl Id for u8 {}
impl Id for u16 {}
impl Id for u32 {}
impl Id for u64 {}
impl Id for usize {}

/// Specifies the number of possible values for a type.
pub enum IdChoices {
    /// If the number is "Small", then the exact number can be specified.
    /// Types which combine IDs should perform the appropriate arithmetic
    /// to determine the number of combined choices.
    Small(usize),

    /// "Large" means that the number is too large to consider.
    /// This indicates that the type is not feasible for exhaustive enumeration.
    Large,
}

/// An ID type with only one possible value.
/// Useful for representing a singleton in a model.
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

/// A map which associates values with IDs in the order of appearance.
///
/// This is useful in creating [`crate::mapping`]s, where real-world values
/// get assigned IDs in the order they are encountered, allowing a small model
/// to be mapped onto a larger real-world system.
///
/// ```
/// use polestar::prelude::*;
///
/// let mut m = IdMap::<_, UpTo<3>>::default();
/// assert_eq!(m.lookup("c"), Ok(UpTo::new(0)));
/// assert_eq!(m.lookup("m"), Ok(UpTo::new(1)));
/// assert_eq!(m.lookup("c"), Ok(UpTo::new(0)));
/// assert_eq!(m.lookup("y"), Ok(UpTo::new(2)));
/// assert_eq!(m.lookup("y"), Ok(UpTo::new(2)));
/// assert!(m.lookup("k").is_err());
/// ```
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
    /// Get the ID for a value, or assign a new ID if the value has not been looked up before.
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
