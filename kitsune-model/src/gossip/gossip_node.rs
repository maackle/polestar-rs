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

use super::scheduler::{Schedule, ScheduleKv};

const SUCCESS_TICKS: u8 = 1;
const ERROR_TICKS: u8 = 2;

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

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, derive_more::Deref, derive_more::DerefMut)]
pub struct ScheduleTimed<N: Id>(ScheduleKv<N, PeerState>);

impl<N: Id> ScheduleTimed<N> {
    pub fn insert_timed(&mut self, n: N, peer: PeerState) -> bool {
        let time = match peer {
            PeerState::Ready => None,
            PeerState::Stale => None,
            PeerState::Active => None,
            PeerState::Closed(outcome) => Some(outcome.ticks()),
        };
        self.0.insert_kv(time, n, peer)
    }
}

/// The state of a single node
#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::IntoIterator)]
pub struct NodeState<N: Id> {
    schedule: ScheduleTimed<N>,
}

impl<N: Id> NodeState<N> {
    pub fn new(peers: impl IntoIterator<Item = N>) -> Self {
        let mut schedule = ScheduleTimed::default();
        for peer in peers {
            schedule.insert_kv(None, peer, PeerState::default());
        }
        Self { schedule }
    }
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

impl GossipOutcome {
    pub fn ticks(&self) -> u8 {
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
    type Action = NodeAction<N>;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        match action {
            NodeAction::Tick => {
                if let Some((n, peer)) = state.schedule.pop() {
                    match peer {
                        PeerState::Ready => {
                            unreachable!()
                        }
                        PeerState::Stale => {
                            unreachable!()
                        }
                        PeerState::Active => {
                            let outcome = GossipOutcome::Failure(FailureReason::Timeout);
                            state.schedule.insert_timed(n, PeerState::Closed(outcome));
                        }
                        PeerState::Closed(_) => {
                            state.schedule.insert_timed(n, PeerState::Ready);
                        }
                    }
                }
            }
            NodeAction::AddPeer(peer) => {
                if !state.schedule.insert_timed(peer, PeerState::default()) {
                    bail!("peer {peer} already exists");
                }
            }
            NodeAction::Incoming { from, msg } => {
                let peer = state.schedule.remove_key(&from).ok_or(anyhow!("no key"))?;
                match msg {
                    Msg::Initiate => match peer {
                        PeerState::Active => bail!("node {from} already in a gossip round"),
                        PeerState::Closed(_) => {
                            bail!("too soon to be initiated with")
                        }
                        _ => {
                            state.schedule.insert_timed(from, PeerState::Active);
                        }
                    },
                    Msg::Complete(new_data) => match peer {
                        PeerState::Active => {
                            state.schedule.insert_timed(
                                from,
                                PeerState::Closed(GossipOutcome::Success(new_data)),
                            );
                        }
                        _ => bail!("node {from} not in a gossip round"),
                    },
                }
            }
            NodeAction::Error { from } => {
                let peer = state.schedule.remove_key(&from).ok_or(anyhow!("no key"))?;
                match peer {
                    PeerState::Active => {
                        state.schedule.insert_timed(
                            from,
                            PeerState::Closed(GossipOutcome::Failure(FailureReason::Protocol)),
                        );
                    }
                    _ => bail!("node {from} not in a gossip round"),
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
pub struct NodeStateUnscheduled<N: Id>(BTreeMap<N, PeerState>);

impl<N: Id> From<NodeState<N>> for NodeStateUnscheduled<N> {
    fn from(state: NodeState<N>) -> Self {
        Self(
            state
                .schedule
                .0
                .into_iter()
                .map(|(_, (n, peer))| (n, peer))
                .collect(),
        )
    }
}

impl<N: Id> Display for NodeState<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (_, (n, peer)) in self.schedule.iter() {
            match peer {
                PeerState::Closed(GossipOutcome::Success(_)) => writeln!(f, "{n}: Success")?,
                PeerState::Closed(GossipOutcome::Failure(reason)) => {
                    writeln!(f, "{n}: Failure({reason:?})")?
                }
                _ => writeln!(f, "{n}: {:?}", peer)?,
            }
        }
        Ok(())
    }
}

impl<N: Id> Display for NodeStateUnscheduled<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (n, peer) in self.0.iter() {
            match peer {
                PeerState::Closed(GossipOutcome::Success(_)) => writeln!(f, "{n}: Success")?,
                PeerState::Closed(GossipOutcome::Failure(reason)) => {
                    writeln!(f, "{n}: Failure({reason:?})")?
                }
                _ => writeln!(f, "{n}: {:?}", peer)?,
            }
        }
        Ok(())
    }
}

impl<N: Id> Display for NodeAction<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeAction::Tick => write!(f, "Tick"),
            NodeAction::AddPeer(n) => write!(f, "AddPeer({n})"),
            NodeAction::Incoming { from, msg } => match msg {
                Msg::Complete(_) => write!(f, "Complete << {from}"),
                _ => write!(f, "{msg:?} << {from}"),
            },
            NodeAction::Error { from } => write!(f, "Error({from})"),
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

    #[test]
    #[ignore = "diagram"]
    fn diagram() {
        // type N = IdUnit;
        type N = UpTo<2>;

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
            |s| Some(NodeStateUnscheduled::from(s)),
            // Some,
            |a| (!matches!(a, NodeAction::Tick)).then_some(a),
        );
    }
}
