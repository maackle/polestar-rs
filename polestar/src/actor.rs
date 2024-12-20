// Adapted from https://github.com/holochain/holochain/blob/ded3a663c9065d998d9f20d2a821836335467a4b/crates/stef/src/share.rs#L0-L1

use std::{fmt::Debug, sync::Arc};

use parking_lot::RwLock;
use proptest::prelude::{Arbitrary, BoxedStrategy, Strategy};

use crate::Machine;

/// Wrap a Fsm in a mutex for shared access.
///
/// It's intentionally not Cloneable, because we intend
/// there to be only a single writer. There can be many readers,
/// via the [`ActorRead`] wrapper.
// TODO: revisit the question of cloneability
#[derive(Default)]
pub struct ShareRw<S>(Arc<RwLock<S>>);

#[derive(Default, Clone, PartialEq, Eq, Hash, derive_more::From)]
pub struct ShareRead<S>(ShareRw<S>);

impl<S> Clone for ShareRw<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S> PartialEq for ShareRw<S> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<S> Eq for ShareRw<S> {}

impl<S: Debug> Debug for ShareRw<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.read(|s| f.debug_tuple("ActorRw").field(s).finish())
    }
}

impl<S: std::hash::Hash> std::hash::Hash for ShareRw<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.read(|s| s.hash(state))
    }
}

impl<S: Arbitrary + 'static> Arbitrary for ShareRw<S> {
    type Parameters = S::Parameters;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(p: Self::Parameters) -> Self::Strategy {
        S::arbitrary_with(p).prop_map(Self::new).boxed()
    }
}

impl<S> From<S> for ShareRw<S> {
    fn from(s: S) -> Self {
        Self::new(s)
    }
}

impl<S> From<S> for ShareRead<S> {
    fn from(s: S) -> Self {
        Self::new(s)
    }
}

impl<S> ShareRead<S> {
    /// Constructor
    pub fn new(s: S) -> Self {
        ShareRw::new(s).into()
    }

    /// Acquire read-only access to the shared state.
    pub fn read<R>(&self, f: impl FnOnce(&S) -> R) -> R {
        self.0.read(f)
    }
}

impl<S> ShareRw<S> {
    /// Constructor
    pub fn new(s: S) -> Self {
        Self(Arc::new(RwLock::new(s)))
    }

    /// Acquire read-only access to the shared state.
    pub fn read<R>(&self, f: impl FnOnce(&S) -> R) -> R {
        f(&*self.0.read())
    }

    /// Acquire write access to the shared state.
    pub fn write<R>(&mut self, f: impl FnOnce(&mut S) -> R) -> R {
        f(&mut *self.0.write())
    }
}

impl<S: Machine + Clone> ShareRw<S> {
    /// Return a cloned copy of the shared state
    pub fn get(&self) -> S {
        let g = self.0.read();
        g.clone()
    }
}

#[derive(Default, Debug, PartialEq, Eq, Hash, derive_more::Deref)]
pub struct Actor<S>(ShareRw<Option<S>>);

impl<S> Clone for Actor<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S> From<S> for Actor<S> {
    fn from(s: S) -> Self {
        Self(ShareRw::new(Some(s)))
    }
}

impl<S> Actor<S> {
    pub fn new(s: S) -> Self {
        Self::from(s)
    }

    /// Acquire read-only access to the shared state.
    pub fn read<R>(&self, f: impl FnOnce(&S) -> R) -> R {
        self.0.read(|x| x.as_ref().map(f).unwrap())
    }
}
