use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fmt::Debug,
    sync::Arc,
};

use polestar::actor::ActorRw;
use rand::Rng;

use super::*;

#[derive(Clone, Debug, derive_more::Deref, derive_more::DerefMut)]
pub struct Panopticon {
    #[deref]
    #[deref_mut]
    pub nodes: HashMap<NodeId, Node>,
}

#[derive(Clone, Debug)]
pub struct NodeState {
    agents: Vec<Agent>,
    vault: BTreeMap<OpHash, OpData>,
    cache: HashMap<OpHash, Op>,
    fetchpool: VecDeque<(OpHash, Option<Peer>, FetchDestination)>,
    peers: Vec<Peer>,
}

impl NodeState {
    pub fn new() -> Self {
        Self {
            agents: vec![],
            vault: Default::default(),
            cache: Default::default(),
            fetchpool: VecDeque::new(),
            peers: vec![],
        }
    }

    pub fn num_integrated(&self) -> usize {
        self.vault
            .values()
            .filter(|v| v.state == OpState::Integrated)
            .count()
    }
}

#[derive(Clone, Debug)]
pub enum NodeEvent {
    AuthorOp(usize),
    AddPeer(Peer),
    StoreOp(Op, FetchDestination),
    SetOpState(OpHash, OpState),
    EnqueueFetch(OpHash, Option<Peer>, FetchDestination),
}

#[derive(Clone, Debug)]
pub enum FetchDestination {
    Vault,
    Cache,
}

/// A node in the network
#[derive(Clone, derive_more::From, derive_more::Deref)]
pub struct Node {
    id: NodeId,
    #[deref]
    state: ActorRw<NodeState>,
}

impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node").field("id", &self.id).finish()
    }
}

/// A node from another node's perspective
#[derive(Clone, Debug, derive_more::From, derive_more::Deref)]
pub struct Peer(Node);

pub trait PeerInterface {
    fn publish_to(&mut self, from: Peer, op: OpHash);
    fn gossip_to(&mut self, from: Peer, ops: Vec<OpHash>);
    fn fetch_from(&self, op: OpHash) -> Option<Op>;
}

impl Node {
    pub fn new(id: NodeId, state: NodeState) -> Self {
        Self {
            id,
            state: ActorRw::new(state),
        }
    }

    pub fn handle_event(&mut self, event: NodeEvent) {
        tracing::info!("node {:?}  event {:?}", self.id, event);
        self.state.write(|n| match event {
            NodeEvent::AuthorOp(num_deps) => n.author(num_deps),
            NodeEvent::AddPeer(peer) => n.peers.push(peer),
            NodeEvent::StoreOp(op, destination) => n.store(op, destination),
            NodeEvent::SetOpState(hash, state) => n.vault.get_mut(&hash).unwrap().state = state,
            NodeEvent::EnqueueFetch(hash, peer, destination) => {
                n.fetchpool.push_back((hash, peer, destination));
            }
        });
    }
}

impl NodeState {
    fn author(&mut self, num_deps: usize) {
        let hash: OpHash = Id::new().into();
        let deps: Vec<OpHash> = self
            .vault
            .range(&hash..)
            .take(num_deps)
            .map(|(hash, _)| hash)
            .cloned()
            .collect();
        let op = Op::new(hash, deps);
        self.vault.insert(
            OpHash::from(&op),
            OpData {
                op,
                state: OpState::Pending(OpOrigin::Authored),
            },
        );
    }

    fn store(&mut self, op: Op, destination: FetchDestination) {
        match destination {
            FetchDestination::Vault => {
                self.vault.insert(
                    op.hash.clone(),
                    OpData {
                        op,
                        state: OpState::Pending(OpOrigin::Fetched),
                    },
                );
            }
            FetchDestination::Cache => {
                self.cache.insert(op.hash.clone(), op);
            }
        }
    }
}

impl Peer {
    fn publish_to(&mut self, from: Peer, op: OpHash) {
        self.0.handle_event(NodeEvent::EnqueueFetch(
            op,
            Some(from),
            FetchDestination::Vault,
        ));
    }

    fn gossip_to(&mut self, from: Peer, ops: Vec<OpHash>) {
        for op in ops {
            self.0.handle_event(NodeEvent::EnqueueFetch(
                op,
                Some(from.clone()),
                FetchDestination::Vault,
            ));
        }
    }

    fn fetch_from(&self, op: OpHash) -> Option<Op> {
        self.0.read(|n| {
            n.vault
                .get(&op)
                .filter(|op_data| op_data.state == OpState::Integrated)
                .map(|op_data| op_data.op.clone())
        })
    }
}

pub fn step(node: Node, t: usize) {
    node.clone().state.write(|n| {
        // handle some fetchpool item
        for _ in 0..10 {
            if let Some((hash, from, destination)) = n.fetchpool.pop_front() {
                // If no "from" specified, pick a random peer
                let peer = from.unwrap_or_else(|| {
                    n.peers[rand::thread_rng().gen_range(0..n.peers.len())].clone()
                });
                if let Some(op) = peer.fetch_from(hash.clone()) {
                    tracing::trace!("node {:?}    fetched {:?}", node.id, hash);
                    n.store(op, destination);
                }
            } else {
                break;
            }
        }

        // move ops through the validation pipeline
        let mut to_validate: Vec<Op> = vec![];
        for op in n.vault.values_mut() {
            match &op.state {
                OpState::Pending(_) => {
                    to_validate.push(op.op.clone());
                }
                OpState::MissingDeps(deps) => {
                    n.fetchpool.extend(
                        deps.iter()
                            .map(|dep| (dep.clone(), None, FetchDestination::Cache)),
                    );
                }
                OpState::Validated => {
                    op.state = OpState::Integrated;

                    // publishing as soon as integrated
                    for peer in n.peers.iter_mut() {
                        let hash = op.op.hash.clone();
                        peer.read(|p| {
                            tracing::trace!(
                                "node {:?}  published {:?}  to {:?}",
                                node.id,
                                hash,
                                peer.id.clone()
                            )
                        });
                        peer.publish_to(node.clone().into(), hash);
                    }
                }
                _ => {}
            }
        }

        for op in to_validate {
            if op
                .deps
                .iter()
                .all(|dep| n.vault.contains_key(dep) || n.cache.contains_key(dep))
            {
                n.vault.get_mut(&op.hash).unwrap().state = OpState::Validated;
            } else {
                n.vault.get_mut(&op.hash).unwrap().state = OpState::MissingDeps(op.deps);
            }
        }

        // gossip ops
        if t % 10 == 0 {
            let ops: Vec<OpHash> = n
                .vault
                .values()
                .filter(|op| op.state == OpState::Integrated)
                .map(|op| OpHash::from(&op.op))
                .collect();
            for peer in n.peers.iter_mut() {
                peer.read(|p| {
                    tracing::trace!(
                        "node {:?}   gossiped {} ops to {:?}",
                        node.id,
                        ops.len(),
                        peer.id.clone()
                    )
                });
                peer.gossip_to(node.clone().into(), ops.clone());
            }
        }
    })
}

#[derive(Clone, Debug)]
pub struct OpData {
    op: Op,
    state: OpState,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpState {
    Pending(OpOrigin),
    Validated,
    MissingDeps(Vec<OpHash>),
    Rejected(String),
    Integrated,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpOrigin {
    Authored,
    Fetched,
}

#[cfg(test)]
mod tests {

    use rand::Rng;
    use system::{Node, NodeEvent, NodeState};

    use super::*;

    #[test]
    fn test_node() {
        tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

        const N: usize = 3;
        const AUTHORED_OPS: usize = 10;
        const MAX_ITERS: usize = 100_000;

        let mut nodes: Vec<Node> = std::iter::repeat_with(NodeState::new)
            .map(|s| Node::new(Id::new().into(), s))
            .take(N)
            .collect();

        // peer discovery
        for i in 0..N {
            let peer = nodes[(i + 1) % N].clone().into();
            nodes[i].handle_event(NodeEvent::AddPeer(peer));
        }

        for i in 0..AUTHORED_OPS {
            nodes[0].handle_event(NodeEvent::AuthorOp(rand::thread_rng().gen_range(0..i + 1)));
        }

        for t in 0..MAX_ITERS {
            for n in nodes.iter() {
                step(n.clone(), t);
            }

            if t % 10 == 0
                && nodes
                    .iter()
                    .all(|n| n.read(|n| n.num_integrated()) == AUTHORED_OPS)
            {
                println!("consistency reached, t = {t}");
                break;
            }
        }

        for n in nodes.iter() {
            assert_eq!(n.read(|n| n.num_integrated()), AUTHORED_OPS);
        }
    }
}
