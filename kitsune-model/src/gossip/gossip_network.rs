use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Display},
};

use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use polestar::{ext::MapExt, id::Id, prelude::*};
use serde::{Deserialize, Serialize};

use super::gossip_node::*;

/*                   █████     ███
                    ░░███     ░░░
  ██████    ██████  ███████   ████   ██████  ████████
 ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
  ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
 ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
░░████████░░██████   ░░█████  █████░░██████  ████ █████
 ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░   */

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Exhaustive, Serialize, Deserialize)]
pub struct GossipAction<N: Id>(pub N, pub NodeAction<N>);

/*        █████               █████
         ░░███               ░░███
  █████  ███████    ██████   ███████    ██████
 ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
░░█████   ░███      ███████   ░███    ░███████
 ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
 ██████   ░░█████ ░░████████  ░░█████ ░░██████
░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░  */

/// The panoptic state of the whole network
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, derive_more::Constructor)]
pub struct GossipState<N: Id> {
    pub nodes: BTreeMap<N, NodeState<N>>,
}

/*                                  █████       ███
                                   ░░███       ░░░
 █████████████    ██████    ██████  ░███████   ████  ████████    ██████
░░███░░███░░███  ░░░░░███  ███░░███ ░███░░███ ░░███ ░░███░░███  ███░░███
 ░███ ░███ ░███   ███████ ░███ ░░░  ░███ ░███  ░███  ░███ ░███ ░███████
 ░███ ░███ ░███  ███░░███ ░███  ███ ░███ ░███  ░███  ░███ ░███ ░███░░░
 █████░███ █████░░████████░░██████  ████ █████ █████ ████ █████░░██████
░░░░░ ░░░ ░░░░░  ░░░░░░░░  ░░░░░░  ░░░░ ░░░░░ ░░░░░ ░░░░ ░░░░░  ░░░░░░  */

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GossipMachine<N: Id> {
    node_machine: NodeMachine<N>,
}

impl<N: Id> Machine for GossipMachine<N> {
    type State = GossipState<N>;
    type Action = GossipAction<N>;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(
        &self,
        mut state: Self::State,
        GossipAction(node, action): Self::Action,
    ) -> TransitionResult<Self> {
        // TODO: probably need to be smarter about what actually happens here.
        match action {
            NodeAction::AddPeer(peer) if node == peer => {
                bail!("node cannot add itself as a peer");
            }
            action => state
                .nodes
                .owned_update(node, |_, node| self.node_machine.transition(node, action))?,
        };
        Ok((state, ()))
    }

    fn is_terminal(&self, s: &Self::State) -> bool {
        false
    }
}

impl<N: Id> GossipMachine<N> {
    pub fn new() -> Self {
        Self {
            node_machine: NodeMachine::new(),
        }
    }

    pub fn initial(&self) -> GossipState<N>
    where
        N: Exhaustive,
    {
        GossipState::new(
            N::iter_exhaustive(None)
                .map(|n| {
                    (
                        n,
                        NodeState::new(N::iter_exhaustive(None).filter(|p| p != &n)),
                    )
                })
                .collect(),
        )
    }
}

/*█████                      █████
 ░░███                      ░░███
 ███████    ██████   █████  ███████    █████
░░░███░    ███░░███ ███░░  ░░░███░    ███░░
  ░███    ░███████ ░░█████   ░███    ░░█████
  ░███ ███░███░░░   ░░░░███  ░███ ███ ░░░░███
  ░░█████ ░░██████  ██████   ░░█████  ██████
   ░░░░░   ░░░░░░  ░░░░░░     ░░░░░  ░░░░░░*/

#[cfg(test)]
mod tests {

    use super::*;
    use itertools::Itertools;
    use polestar::{
        diagram::exhaustive::*,
        id::{IdUnit, UpTo},
        machine::checked::Predicate as P,
        traversal::traverse_checked,
    };

    #[test]
    #[ignore = "diagram"]
    fn diagram() {}
}
