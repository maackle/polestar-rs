use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Display},
};

use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use polestar::{ext::MapExt, id::Id, prelude::*};
use serde::{Deserialize, Serialize};

/*                   █████     ███
                    ░░███     ░░░
  ██████    ██████  ███████   ████   ██████  ████████
 ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
  ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
 ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
░░████████░░██████   ░░█████  █████░░██████  ████ █████
 ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░   */

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Exhaustive, Serialize, Deserialize)]
pub enum NodeAction<N: Id> {
    /// Tick the clock
    Tick,
    /// Add a peer to the node's peer set
    AddPeer(N),
    /// Receive a message from another node
    Incoming { from: N, msg: Msg },
    /// Simulate a protocol error
    Error { from: N },
    // /// Hang up on another node without telling them
    // Hangup { to: N },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Exhaustive, Serialize, Deserialize)]
pub enum Msg {
    /// Initiate a gossip round
    Initiate,
    /// Complete a gossip round successfully, indicating whether new data was sent
    Complete(bool),
}

/*        █████               █████
         ░░███               ░░███
  █████  ███████    ██████   ███████    ██████
 ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
░░█████   ░███      ███████   ░███    ░███████
 ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
 ██████   ░░█████ ░░████████  ░░█████ ░░██████
░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░  */

/// The state of a single node
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, derive_more::From)]
pub struct NodeState<N: Id> {
    peers: BTreeMap<N, PeerState>,
    // rounds: BTreeSet<N>,
}

/// The state of a peer from the perspective of another
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, derive_more::From)]
pub struct PeerState {
    active_round: bool,
    last_outcome: Option<(GossipOutcome, usize)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GossipOutcome {
    /// The last gossip attempt was successful.
    /// If true, new data was received. If false, nodes were already in sync.
    Success(bool),
    /// The last gossip attempt failed due to timeout or protocol error.
    Failure,
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
pub struct NodeMachine<N: Id> {
    fully_connected_peers: BTreeSet<N>,
}

impl<N: Id> Machine for NodeMachine<N> {
    type State = NodeState<N>;
    type Action = NodeAction<N>;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        match action {
            NodeAction::Tick => state.peers.values_mut().for_each(|peer| {
                // TODO: timeout
                peer.last_outcome.as_mut().map(|(_, time)| *time += 1);
            }),
            NodeAction::AddPeer(peer) => {
                if let Some(_) = state.peers.insert(peer, PeerState::default()) {
                    bail!("peer {peer} already exists");
                }
            }
            NodeAction::Incoming { from, msg } => match msg {
                Msg::Initiate => state.peers.owned_update(from, |peers, mut peer| {
                    if peer.active_round {
                        bail!("node {from} already in a gossip round");
                    } else {
                        peer.active_round = true;
                    }
                    Ok((peer, ()))
                })?,
                Msg::Complete(new_data) => state.peers.owned_update(from, |peers, mut peer| {
                    if peer.active_round {
                        peer.active_round = false;
                        peer.last_outcome = Some((GossipOutcome::Success(new_data), 0));
                    } else {
                        bail!("node {from} not in a gossip round");
                    }
                    Ok((peer, ()))
                })?,
            },
            NodeAction::Error { from } => state.peers.owned_update(from, |peers, mut peer| {
                if peer.active_round {
                    peer.active_round = false;
                    peer.last_outcome = Some((GossipOutcome::Failure, 0));
                } else {
                    bail!("node {from} not in a gossip round");
                }
                Ok((peer, ()))
            })?, // NodeAction::Hangup { to } => {
                 //     state.peers.owned_update(to, |peers, mut peer| {
                 //         peer.active_round = false;
                 //         peer.last_outcome = None;
                 //         Ok((peer, ()))
                 //     })?
                 // }
        }
        Ok((state, ()))
    }

    fn is_terminal(&self, s: &Self::State) -> bool {
        false
    }
}

impl<N: Id + Exhaustive> NodeMachine<N> {
    pub fn new() -> Self {
        Self {
            fully_connected_peers: N::iter_exhaustive(None).collect(),
        }
    }

    pub fn initial(&self) -> NodeState<N> {
        NodeState::default()
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
    };

    #[test]
    #[ignore = "diagram"]
    fn diagram() {
        type N = IdUnit;

        let machine = NodeMachine::<N>::new();
        let state = machine.initial();

        write_dot_state_diagram_mapped(
            "gossip-node.dot",
            machine,
            state,
            &DiagramConfig {
                max_depth: Some(3),
                ..Default::default()
            },
            |state| {
                Some({
                    state
                        .peers
                        .iter()
                        .map(|(n, peer)| {
                            format!("{n}: {} {:?}", peer.active_round, peer.last_outcome)
                        })
                        .collect_vec()
                        .join("\n")
                })
            },
            |action| {
                Some(match action {
                    NodeAction::Tick => "Tick".to_string(),
                    NodeAction::AddPeer(n) => format!("AddPeer({n})"),
                    NodeAction::Incoming { from, msg } => format!("{msg:?} <- {from}"),
                    NodeAction::Error { from } => format!("Error({from})"),
                })
            },
        );
    }
}
