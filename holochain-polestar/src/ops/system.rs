use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::Arc,
};

use polestar::actor::ActorRw;
use rand::Rng;

use super::*;

pub struct Nodes {
    nodes: HashSet<NodeId, Node>,
}

pub struct NodeState {
    id: NodeId,
    agents: Vec<Agent>,
    vault: BTreeMap<OpHash, OpData>,
    cache: HashMap<OpHash, Op>,
    fetchpool: VecDeque<(OpHash, Option<Peer>, FetchDestination)>,
    peers: Vec<Peer>,
}

impl NodeState {
    pub fn new() -> Self {
        Self {
            id: Id::new().into(),
            agents: vec![],
            vault: Default::default(),
            cache: Default::default(),
            fetchpool: VecDeque::new(),
            peers: vec![],
        }
    }
}

pub enum NodeEvent {
    AuthorOp(usize),
    StoreOp(Op, FetchDestination),
    SetOpState(OpHash, OpState),
    EnqueueFetch(OpHash, Peer, FetchDestination),
}

pub enum FetchDestination {
    Vault,
    Cache,
}

/// A node in the network
#[derive(Clone, derive_more::From, derive_more::Deref)]
pub struct Node(ActorRw<NodeState>);

/// A node from another node's perspective
#[derive(Clone, derive_more::From, derive_more::Deref)]
pub struct Peer(Node);

pub trait PeerInterface {
    fn publish_to(&mut self, from: Peer, op: OpHash);
    fn gossip_to(&mut self, from: Peer, ops: Vec<OpHash>);
    fn fetch_from(&self, op: OpHash) -> Option<Op>;
}

impl Node {
    pub fn new(state: NodeState) -> Self {
        Self(ActorRw::new(state))
    }

    pub fn handle_event(&self, event: NodeEvent) {
        self.0.write(|n| todo!());
    }
}

impl Peer {
    fn publish_to(&self, from: Peer, op: OpHash) {
        self.0.write(|n| {
            n.fetchpool
                .push_back((op, Some(from), FetchDestination::Vault))
        });
    }

    fn gossip_to(&self, from: Peer, ops: Vec<OpHash>) {
        self.0.write(|n| {
            n.fetchpool.extend(
                ops.into_iter()
                    .map(|op| (op, Some(from.clone()), FetchDestination::Vault)),
            );
        });
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

impl NodeState {
    pub fn author(&mut self, num_deps: usize) {
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

    pub fn store(&mut self, op: Op, destination: FetchDestination) {
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

pub fn step(node: Node, t: usize) {
    node.clone().write(|n| {
        // handle some fetchpool item
        for _ in 0..10 {
            if let Some((hash, from, destination)) = n.fetchpool.pop_front() {
                // If no "from" specified, pick a random peer
                let peer = from.unwrap_or_else(|| {
                    n.peers[rand::thread_rng().gen_range(0..n.peers.len())].clone()
                });
                if let Some(op) = peer.fetch_from(hash.clone()) {
                    tracing::trace!("node {:?}    fetched {:?}", n.id, hash);
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
                    for peer in n.peers.iter() {
                        let hash = op.op.hash.clone();
                        peer.read(|p| {
                            tracing::trace!(
                                "node {:?}  published {:?}  to {:?}",
                                n.id,
                                hash,
                                p.id.clone()
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
            for peer in n.peers.iter() {
                peer.read(|p| {
                    tracing::trace!(
                        "node {:?}   gossiped {} ops to {:?}",
                        n.id,
                        ops.len(),
                        p.id.clone()
                    )
                });
                peer.gossip_to(node.clone().into(), ops.clone());
            }
        }
    })
}

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

#[test]
fn test_node() {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

    const N: usize = 3;
    const AUTHORED_OPS: usize = 100;
    const MAX_ITERS: usize = 100_000;

    let nodes: Vec<Node> = std::iter::repeat_with(NodeState::new)
        .map(Node::new)
        .take(N)
        .collect();

    // peer discovery
    for i in 0..N {
        nodes[i].write(|n| {
            n.peers.push(nodes[(i + 1) % N].clone().into());
        });
    }

    for i in 0..AUTHORED_OPS {
        nodes[0].write(|n| n.author(rand::thread_rng().gen_range(0..i + 1)));
    }

    for t in 0..MAX_ITERS {
        for n in nodes.iter() {
            step(n.clone(), t);
        }

        if t % 100 == 0
            && nodes
                .iter()
                .all(|n| n.read(|n| n.vault.len()) == AUTHORED_OPS)
        {
            println!("consistency reached, t = {t}");
            break;
        }
    }

    for n in nodes.iter() {
        assert_eq!(n.read(|n| n.vault.len()), AUTHORED_OPS);
    }
}
