use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Display},
    marker::PhantomData,
};

use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use itertools::Itertools;
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
pub enum PeerState {
    #[default]
    Ready,
    Stale,
    Active,
    Closed(GossipOutcome),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GossipOutcome {
    /// The last gossip attempt was successful.
    /// If true, new data was received. If false, nodes were already in sync.
    Success(bool),
    /// The last gossip attempt failed due to timeout or protocol error.
    Failure(FailureReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FailureReason {
    Timeout,
    Protocol,
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
    phantom: PhantomData<N>,
}

impl<N: Id> Machine for NodeMachine<N> {
    type State = NodeState<N>;
    type Action = NodeAction<N>;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        match action {
            NodeAction::Tick => state.peers.values_mut().for_each(|peer| match peer {
                PeerState::Ready => {
                    *peer = PeerState::Stale;
                }
                PeerState::Stale => {}
                PeerState::Active => {
                    *peer = PeerState::Closed(GossipOutcome::Failure(FailureReason::Timeout))
                }
                PeerState::Closed(_) => {
                    *peer = PeerState::Ready;
                    // if *time <= outcome.ticks(self) {
                    //     *time += 1;
                    // } else {
                    //     // TODO: remove node?
                    //     *peer = PeerState::Stale;
                    // }
                }
            }),
            NodeAction::AddPeer(peer) => {
                if let Some(_) = state.peers.insert(peer, PeerState::default()) {
                    bail!("peer {peer} already exists");
                }
            }
            NodeAction::Incoming { from, msg } => match msg {
                Msg::Initiate => state.peers.owned_update(from, |_, mut peer| {
                    match peer {
                        PeerState::Active => bail!("node {from} already in a gossip round"),
                        PeerState::Closed(_) => {
                            bail!("too soon to be initiated with")
                        }
                        _ => peer = PeerState::Active,
                    }
                    Ok((peer, ()))
                })?,
                Msg::Complete(new_data) => state.peers.owned_update(from, |_, mut peer| {
                    match peer {
                        PeerState::Active => {
                            peer = PeerState::Closed(GossipOutcome::Success(new_data));
                        }
                        _ => bail!("node {from} not in a gossip round"),
                    }
                    Ok((peer, ()))
                })?,
            },
            NodeAction::Error { from } => state.peers.owned_update(from, |_, mut peer| {
                match peer {
                    PeerState::Active => {
                        peer = PeerState::Closed(GossipOutcome::Failure(FailureReason::Protocol));
                    }
                    _ => bail!("node {from} not in a gossip round"),
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

    fn is_terminal(&self, _s: &Self::State) -> bool {
        false
    }
}

impl<N: Id> NodeMachine<N> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
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

pub fn state_display<N: Id>(state: NodeState<N>) -> Option<String> {
    Some({
        state
            .peers
            .iter()
            .map(|(n, peer)| match peer {
                PeerState::Closed(GossipOutcome::Success(_)) => {
                    format!("{n}: Success")
                }
                PeerState::Closed(GossipOutcome::Failure(reason)) => {
                    format!("{n}: Failure({reason:?})")
                }
                _ => format!("{n}: {:?}", peer),
            })
            .collect_vec()
            .join("\n")
    })
}

pub fn action_display<N: Id>(action: NodeAction<N>) -> Option<String> {
    Some(match action {
        NodeAction::Tick => "Tick".to_string(),
        NodeAction::AddPeer(n) => format!("AddPeer({n})"),
        NodeAction::Incoming { from, msg } => match msg {
            Msg::Complete(_) => format!("Complete <- {from}"),
            _ => format!("{msg:?} <- {from}"),
        },
        NodeAction::Error { from } => format!("Error({from})"),
    })
}

#[cfg(test)]
mod tests {

    use super::*;
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
                max_depth: None,
                ..Default::default()
            },
            state_display,
            action_display,
        );
    }
}
