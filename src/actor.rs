// Adapted from https://github.com/holochain/holochain/blob/ded3a663c9065d998d9f20d2a821836335467a4b/crates/stef/src/share.rs#L0-L1

use std::{fmt::Debug, sync::Arc};

use parking_lot::RwLock;
use proptest::prelude::{Arbitrary, BoxedStrategy, Strategy};

use crate::{fsm::FsmResult, Fsm};

/// Wrap a Fsm in a mutex for shared access.
///
/// It's intentionally not Cloneable, because we intend
/// there to be only a single writer. There can be many readers,
/// via the [`ActorRead`] wrapper.
// TODO: revisit the question of cloneability
#[derive(Default)]
pub struct ActorRw<S>(Arc<RwLock<S>>);

#[derive(Default, Clone, PartialEq, Eq, Hash, derive_more::From)]
pub struct ActorRead<S>(ActorRw<S>);

impl<S> Clone for ActorRw<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S> PartialEq for ActorRw<S> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<S> Eq for ActorRw<S> {}

impl<S: Debug> Debug for ActorRw<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.read(|s| f.debug_tuple("ActorRw").field(s).finish())
    }
}

impl<S: std::hash::Hash> std::hash::Hash for ActorRw<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.read(|s| s.hash(state))
    }
}

impl<S: Arbitrary + 'static> Arbitrary for ActorRw<S> {
    type Parameters = S::Parameters;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(p: Self::Parameters) -> Self::Strategy {
        S::arbitrary_with(p).prop_map(Self::new).boxed()
    }
}

impl<S> From<S> for ActorRw<S> {
    fn from(s: S) -> Self {
        Self::new(s)
    }
}

impl<S> From<S> for ActorRead<S> {
    fn from(s: S) -> Self {
        Self::new(s)
    }
}

impl<S> ActorRead<S> {
    /// Constructor
    pub fn new(s: S) -> Self {
        ActorRw::new(s).into()
    }

    /// Acquire read-only access to the shared state.
    pub fn read<R>(&self, f: impl FnOnce(&S) -> R) -> R {
        self.0.read(f)
    }
}

impl<S> ActorRw<S> {
    /// Constructor
    pub fn new(s: S) -> Self {
        Self(Arc::new(RwLock::new(s)))
    }

    /// Acquire read-only access to the shared state.
    pub fn read<R>(&self, f: impl FnOnce(&S) -> R) -> R {
        f(&*self.0.read())
    }

    /// Acquire write access to the shared state.
    /// This isn't really ideal since it doesn't hide mutability,
    /// so you have to be careful. Better to use a macro (TODO).
    /// TODO: also is it OK to return values from this? or should it
    /// be mandated that effects have to be handled internally, so that
    /// there is less chance that someone will use this for direct
    /// mutable access?
    pub fn write<R>(&mut self, f: impl FnOnce(&mut S) -> R) -> R {
        f(&mut *self.0.write())
    }
}

impl<S: Fsm + Clone> ActorRw<S> {
    /// Return a cloned copy of the shared state
    pub fn get(&self) -> S {
        let g = self.0.read();
        g.clone()
    }
}

#[derive(Default, Debug, PartialEq, Eq, Hash, derive_more::Deref)]
pub struct ActorFsm<S>(ActorRw<Option<S>>);

impl<S> Clone for ActorFsm<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S> From<S> for ActorFsm<S> {
    fn from(s: S) -> Self {
        Self(ActorRw::new(Some(s)))
    }
}

impl<S: Fsm> Fsm for ActorFsm<S> {
    type Action = S::Action;
    type Fx = S::Fx;
    type Error = S::Error;

    fn transition(self, event: Self::Action) -> FsmResult<Self> {
        let fx = {
            let mut lock = self.0 .0.write();
            let state = std::mem::take(&mut *lock).unwrap();
            let (state, fx) = state.transition(event)?;
            *lock = Some(state);
            fx
        };
        Ok((self, fx))
    }

    fn is_terminal(&self) -> bool {
        self.0
             .0
            .read()
            .as_ref()
            .map(|x| x.is_terminal())
            .unwrap_or(false)
    }
}

impl<S: Fsm> ActorFsm<S> {
    pub fn transition_mut(&mut self, event: S::Action) -> Option<Result<S::Fx, S::Error>> {
        let mut lock = self.0 .0.write();
        match lock.take()?.transition(event) {
            Err(e) => Some(Err(e)),
            Ok((state, fx)) => {
                *lock = Some(state);
                Some(Ok(fx))
            }
        }
    }
}
