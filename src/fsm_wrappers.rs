use crate::Fsm;
use proptest::prelude::{Arbitrary, BoxedStrategy, Strategy};
use proptest_derive::Arbitrary;
use std::{cell::RefCell, collections::HashMap};

/// Use a CellFsm when you want to transition an FSM in-place, via [`CellFsm::transition_mut`].
///
/// After [`CellFsm::transition_mut`] produces an Error, subsequent calls will return None.
/// Thus, it is expected that the cell will be dropped after producing an error.
///
/// ```
/// use polestar::prelude::*;
///
/// struct Inner(u8);
///
/// impl Fsm for Inner {
///     type Event = ();
///     type Fx = ();
///     type Error = Infallible;
///
///     fn transition(self, _: Self::Event) -> FsmResult<Self> {
///         Ok((Self(self.0.wrapping_add(1)), ()))
///     }
/// }
///
/// struct Outer {
///     inner: CellFsm<Inner>,
/// }
///
/// impl Fsm for Outer {
///     type Event = ();
///     type Fx = ();
///     type Error = Infallible;
///
///     fn transition(mut self, _: Self::Event) -> FsmResult<Self> {
///         // This unwrap is safe because if Outer returns an error, the cell will be dropped
///         // and never used again.
///         self.inner.transition_mut(()).unwrap()?;
///         Ok((self, ()))
///     }
/// }
/// ```
#[derive(Clone, derive_more::Deref)]
pub struct CellFsm<S>(RefCell<Option<S>>);

impl<S> CellFsm<S> {
    pub fn new(s: S) -> Self {
        Self(RefCell::new(Some(s)))
    }
}

impl<S> From<S> for CellFsm<S> {
    fn from(s: S) -> Self {
        Self::new(s)
    }
}

impl<S: Fsm> CellFsm<S> {
    pub fn transition_mut(&mut self, event: S::Event) -> Option<Result<S::Fx, S::Error>> {
        match self.0.take()?.transition(event) {
            Err(e) => Some(Err(e)),
            Ok((state, fx)) => {
                self.0.replace(Some(state));
                Some(Ok(fx))
            }
        }
    }
}

impl<S: PartialEq> PartialEq for CellFsm<S> {
    fn eq(&self, other: &Self) -> bool {
        *self.0.borrow() == *other.0.borrow()
    }
}

impl<S: Eq> Eq for CellFsm<S> {}

impl<S: std::fmt::Debug> std::fmt::Debug for CellFsm<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FsmCell").field(&self.0.borrow()).finish()
    }
}

impl<S: std::hash::Hash> std::hash::Hash for CellFsm<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state)
    }
}

impl<S: Arbitrary + 'static> Arbitrary for CellFsm<S> {
    type Parameters = S::Parameters;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(p: Self::Parameters) -> Self::Strategy {
        S::arbitrary_with(p).prop_map(Self::new).boxed()
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Arbitrary,
    derive_more::Deref,
    derive_more::DerefMut,
    derive_more::From,
    derive_more::Into,
)]
pub struct FsmHashMap<K: Eq + std::hash::Hash, V: Fsm>(HashMap<K, V>);

impl<K: Eq + std::hash::Hash, V: Fsm> FsmHashMap<K, V> {
    pub fn transition_mut(&mut self, k: K, event: V::Event) -> Option<Result<V::Fx, V::Error>> {
        let r = self.0.remove(&k)?.transition(event);
        match r {
            Ok((state, fx)) => {
                self.0.insert(k, state);
                Some(Ok(fx))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

impl<K: Eq + std::hash::Hash, V: Fsm> Default for FsmHashMap<K, V> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}
