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

const SUCCESS_TICKS: Timer = 1;
const ERROR_TICKS: Timer = 2;

/*                   █████     ███
                    ░░███     ░░░
  ██████    ██████  ███████   ████   ██████  ████████
 ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
  ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
 ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
░░████████░░██████   ░░█████  █████░░██████  ████ █████
 ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░   */

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Exhaustive, Serialize, Deserialize)]
pub enum NodeAction<N: Id, CompleteStatus> {
    /// Tick the clock
    Tick,
    /// Add a peer to the node's peer set
    AddPeer(N),
    // /// This active peer round was marked as timed-out
    // Timeout(N),
    /// Receive (and accept) a message from another node
    Incoming { from: N, msg: Msg<CompleteStatus> },
    // /// Hang up on another node without telling them
    // Hangup { to: N },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Exhaustive, Serialize, Deserialize)]
pub enum Msg<CompleteStatus> {
    /// Initiate a gossip round
    Initiate,
    /// Receive a valid message, continuing the round (details hidden here)
    Touch,
    /// Receive a message that goes against protocol, causing an error
    Junk,
    /// Complete a gossip round successfully, indicating whether new data was sent
    Complete(CompleteStatus),
}

impl<N: Id> From<NodeAction<N, bool>> for NodeAction<N, IdUnit> {
    fn from(action: NodeAction<N, bool>) -> Self {
        match action {
            NodeAction::Incoming { from, msg } => NodeAction::Incoming {
                from,
                msg: msg.into(),
            },
            NodeAction::Tick => NodeAction::Tick,
            NodeAction::AddPeer(n) => NodeAction::AddPeer(n),
            // NodeAction::Timeout(n) => NodeAction::Timeout(n),
        }
    }
}

impl From<Msg<bool>> for Msg<IdUnit> {
    fn from(msg: Msg<bool>) -> Self {
        match msg {
            Msg::Initiate => Msg::Initiate,
            Msg::Touch => Msg::Touch,
            Msg::Junk => Msg::Junk,
            Msg::Complete(_) => Msg::Complete(IdUnit),
        }
    }
}

/*        █████               █████
         ░░███               ░░███
  █████  ███████    ██████   ███████    ██████
 ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
░░█████   ░███      ███████   ░███    ░███████
 ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
 ██████   ░░█████ ░░████████  ░░█████ ░░██████
░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░  */

pub type Timer = u16;

/// The state of a single node
#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::IntoIterator)]
pub struct NodeState<N: Id> {
    pub peers: BTreeMap<N, PeerState>,
}

impl<N: Id> NodeState<N> {
    pub fn new(peers: impl IntoIterator<Item = N>) -> Self {
        let peers = peers
            .into_iter()
            .map(|n| (n, PeerState::default()))
            .collect();
        Self { peers }
    }

    pub fn set_peer(&mut self, n: N, phase: PeerPhase) -> bool {
        self.peers.insert(n, PeerState::new(phase)).is_none()
    }
}

impl<N: Id> Display for NodeState<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (n, peer) in self.peers.iter() {
            writeln!(f, "{n}: {}", peer)?
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, derive_more::From)]
pub struct PeerState {
    pub phase: PeerPhase,
    pub timer: Timer,
}

impl PeerState {
    pub fn new(phase: PeerPhase) -> Self {
        Self {
            timer: phase.initial_timer(),
            phase,
        }
    }

    /// When the timer expires, the peer transitions to another state.
    pub fn timeout(&self) -> Self {
        if self.timer == 0 {
            Self::new(match self.phase {
                PeerPhase::Ready => PeerPhase::Ready,
                PeerPhase::Active => {
                    PeerPhase::Closed(GossipOutcome::Failure(FailureReason::Timeout))
                }
                PeerPhase::Closed(_) => PeerPhase::Ready,
            })
        } else {
            *self
        }
    }
}

impl Display for PeerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.phase {
            PeerPhase::Closed(GossipOutcome::Success(_)) => {
                write!(f, "Success t={}", self.timer)
            }
            PeerPhase::Closed(GossipOutcome::Failure(reason)) => {
                write!(f, "Failure({reason:?}) t={}", self.timer)
            }
            _ => write!(f, "{:?} t={}", self.phase, self.timer),
        }
    }
}

/// The state of a peer from the perspective of another
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, derive_more::From)]
pub enum PeerPhase {
    #[default]
    Ready,
    Active,
    Closed(GossipOutcome),
}

impl PeerPhase {
    pub fn initial_timer(&self) -> Timer {
        match self {
            Self::Ready => 0,
            Self::Active => 1,
            Self::Closed(outcome) => outcome.ticks(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GossipOutcome {
    /// The last gossip attempt was successful.
    /// If true, new data was received. If false, nodes were already in sync.
    Success(bool),
    /// The last gossip attempt failed due to timeout or protocol error.
    Failure(FailureReason),
}

impl GossipOutcome {
    pub fn ticks(&self) -> Timer {
        match self {
            GossipOutcome::Success(_) => SUCCESS_TICKS,
            GossipOutcome::Failure(_) => ERROR_TICKS,
        }
    }
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
    type Action = NodeAction<N, bool>;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        match action {
            NodeAction::Tick => {
                state.peers.values_mut().for_each(|peer| {
                    if peer.timer == 0 {
                        *peer = peer.timeout();
                    } else {
                        peer.timer = peer.timer.saturating_sub(1);
                    }
                });
            }
            NodeAction::AddPeer(peer) => {
                if !state.set_peer(peer, PeerPhase::default()) {
                    bail!("peer {peer} already exists");
                }
            }
            NodeAction::Incoming { from, msg } => {
                let peer = state.peers.get_mut(&from).ok_or(anyhow!("no key"))?;
                match msg {
                    Msg::Initiate => match peer.phase {
                        PeerPhase::Active => bail!("node {from} already in a gossip round"),
                        PeerPhase::Closed(_) => {
                            bail!("too soon to be initiated with")
                        }
                        _ => {
                            state.set_peer(from, PeerPhase::Active);
                        }
                    },
                    Msg::Touch => match peer.phase {
                        PeerPhase::Active => {
                            peer.timer = peer.phase.initial_timer();
                        }
                        _ => bail!("node {from} not in a gossip round"),
                    },
                    Msg::Junk => match peer.phase {
                        PeerPhase::Active => {
                            state.set_peer(
                                from,
                                PeerPhase::Closed(GossipOutcome::Failure(FailureReason::Protocol)),
                            );
                        }
                        _ => bail!("node {from} not in a gossip round"),
                    },
                    Msg::Complete(new_data) => match peer.phase {
                        PeerPhase::Active => {
                            state.set_peer(
                                from,
                                PeerPhase::Closed(GossipOutcome::Success(new_data)),
                            );
                        }
                        _ => bail!("node {from} not in a gossip round"),
                    },
                }
            }
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

    pub fn initial(&self) -> NodeState<N>
    where
        N: Exhaustive,
    {
        NodeState::new(N::iter_exhaustive(None))
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeStateSimple<N: Id>(BTreeMap<N, String>);

impl<N: Id> NodeStateSimple<N> {
    pub fn new(timed: bool, state: NodeState<N>) -> Self {
        Self(
            state
                .peers
                .into_iter()
                .map(|(n, peer)| {
                    let phase = match peer.phase {
                        PeerPhase::Ready => "Ready",
                        PeerPhase::Active => "Active",
                        PeerPhase::Closed(outcome) => match outcome {
                            GossipOutcome::Success(_) => "Success",
                            GossipOutcome::Failure(_) => "Failure",
                        },
                    };

                    if timed {
                        (n, format!("{phase} t={}", peer.timer))
                    } else {
                        (n, phase.to_string())
                    }
                })
                .collect(),
        )
    }
}

impl<N: Id> Display for NodeStateSimple<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (n, phase) in self.0.iter() {
            writeln!(f, "{n} : {}", phase)?
        }
        Ok(())
    }
}

impl<N: Id, CompleteStatus: Debug + Display> Display for NodeAction<N, CompleteStatus> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeAction::Tick => write!(f, "Tick"),
            NodeAction::AddPeer(n) => write!(f, "AddPeer({n})"),
            NodeAction::Incoming { from, msg } => match msg {
                Msg::Complete(s) => write!(f, "Complete({s}) << {from}"),
                _ => write!(f, "{msg:?} << {from}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use polestar::{
        diagram::exhaustive::*,
        id::{IdUnit, UpTo},
    };
}
