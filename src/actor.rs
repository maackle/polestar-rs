// Adapted from https://github.com/holochain/holochain/blob/ded3a663c9065d998d9f20d2a821836335467a4b/crates/stef/src/share.rs#L0-L1

use std::sync::Arc;

use parking_lot::RwLock;

use crate::fsm::Fsm;

/// Wrap a Fsm in a mutex for shared access.
///
/// It's intentionally not Cloneable, because we intend
/// there to be only a single writer. There can be many readers,
/// via the [`ActorRead`] wrapper.
// TODO: revisit the question of cloneability
#[derive(Default)]
pub struct ActorRw<S>(Arc<RwLock<S>>);

#[derive(Default, Clone, derive_more::From)]
pub struct ActorRead<S>(ActorRw<S>);

impl<S> Clone for ActorRw<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
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
        let g = self.0.read();
        f(&g)
    }

    /// Acquire write access to the shared state.
    /// This isn't really ideal since it doesn't hide mutability,
    /// so you have to be careful. Better to use a macro (TODO).
    /// TODO: also is it OK to return values from this? or should it
    /// be mandated that effects have to be handled internally, so that
    /// there is less chance that someone will use this for direct
    /// mutable access?
    pub fn write<R>(&self, f: impl FnOnce(&mut S) -> R) -> R {
        f(&mut self.0.write())
    }
}

// TODO: even this probably isn't ideal. The actual transition should be applied
// only to the top level state, and may trickle down to this sub-state. With the
// proper "lenses", the substate event can be lifted to the whole-system event.
impl<S: Fsm> ActorRw<S> {
    /// Acquire write access to the shared state to perform a mutation.
    pub fn transition(&self, t: S::Event) -> S::Fx {
        self.transition_with(t, |_| ()).1
    }

    /// Acquire write access to the shared state to perform a mutation,
    /// and do a read on the modified state within the same atomic mutex acquisition.
    pub fn transition_with<R>(&self, t: S::Event, f: impl FnOnce(&S) -> R) -> (R, S::Fx) {
        let mut g = self.0.write();
        let eff = g.transition(t);
        (f(&g), eff)
    }
}

impl<S: Fsm + Clone> ActorRw<S> {
    /// Return a cloned copy of the shared state
    pub fn get(&self) -> S {
        let g = self.0.read();
        g.clone()
    }
}

impl<S: Fsm> Fsm for ActorRw<S> {
    type Event = S::Event;
    type Fx = S::Fx;

    fn transition(&mut self, t: Self::Event) -> Self::Fx {
        ActorRw::transition(self, t)
    }
}

impl<T: Fsm + std::fmt::Debug> std::fmt::Debug for ActorRw<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.read(|s| f.debug_tuple("Share").field(s).finish())
    }
}
