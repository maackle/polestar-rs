//! Extension trait adding traversal entry points to every [`Machine`].

use std::fmt::Debug;

use exhaustive::Exhaustive;
use polestar_core::Machine;

use crate::traversal::Traversal;

/// Adds [`MachineExt::traverse`] to every [`Machine`].
pub trait MachineExt: Machine {
    /// Create a new [`Traversal`] for this machine.
    fn traverse(self, initial: impl IntoIterator<Item = Self::State>) -> Traversal<Self>
    where
        Self::State: Clone + Debug,
        Self::Action: Clone + Debug + Exhaustive,
    {
        Traversal::new(self, initial)
    }
}

impl<M: Machine> MachineExt for M {}
