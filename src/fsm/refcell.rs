use crate::Machine;
use proptest::prelude::{Arbitrary, BoxedStrategy, Strategy};
use std::cell::RefCell;

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
pub struct FsmRefCell<S>(RefCell<Option<S>>);

impl<S> FsmRefCell<S> {
    pub fn new(s: S) -> Self {
        Self(RefCell::new(Some(s)))
    }
}

impl<S> From<S> for FsmRefCell<S> {
    fn from(s: S) -> Self {
        Self::new(s)
    }
}

impl<S: Machine> FsmRefCell<S> {
    pub fn transition_mut(&mut self, event: S::Action) -> Option<Result<S::Fx, S::Error>> {
        match self.0.take()?.transition(event) {
            Err(e) => Some(Err(e)),
            Ok((state, fx)) => {
                self.0.replace(Some(state));
                Some(Ok(fx))
            }
        }
    }
}

impl<S: PartialEq> PartialEq for FsmRefCell<S> {
    fn eq(&self, other: &Self) -> bool {
        *self.0.borrow() == *other.0.borrow()
    }
}

impl<S: Eq> Eq for FsmRefCell<S> {}

impl<S: std::fmt::Debug> std::fmt::Debug for FsmRefCell<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FsmCell").field(&self.0.borrow()).finish()
    }
}

impl<S: std::hash::Hash> std::hash::Hash for FsmRefCell<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state)
    }
}

impl<S: Arbitrary + 'static> Arbitrary for FsmRefCell<S> {
    type Parameters = S::Parameters;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(p: Self::Parameters) -> Self::Strategy {
        S::arbitrary_with(p).prop_map(Self::new).boxed()
    }
}
