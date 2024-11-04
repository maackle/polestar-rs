use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::Arc,
};

use polestar::actor::ActorRw;
use rand::Rng;

use super::*;

pub struct NodeState {
    id: NodeId,
    agents: Vec<Agent>,
    vault: BTreeMap<OpHash, OpData>,
    cache: HashMap<OpHash, Op>,
    fetchpool: VecDeque<(OpHash, Option<Peer>)>,
    peers: Vec<Peer>,
    thunks: VecDeque<(usize, Thunk)>,
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
            thunks: VecDeque::new(),
        }
    }
}

pub enum Thunk {
    FetchOp(OpHash, Peer),
    PublishOp(Op, Peer),
}

pub type Node = ActorRw<NodeState>;

#[derive(Clone, derive_more::From, derive_more::Deref)]
pub struct Peer(Node);

pub trait NodeInterface {
    fn publish_to(&mut self, from: Peer, op: OpHash);
    fn gossip_to(&mut self, from: Peer, ops: Vec<OpHash>);
    fn fetch_from(&self, op: OpHash) -> Option<Op>;
}

impl Peer {
    fn publish_to(&self, from: Peer, op: OpHash) {
        self.0.write(|n| n.fetchpool.push_back((op, Some(from))));
    }

    fn gossip_to(&self, from: Peer, ops: Vec<OpHash>) {
        self.0.write(|n| {
            n.fetchpool
                .extend(ops.into_iter().map(|op| (op, Some(from.clone()))));
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
}

pub fn step(node: Node, t: usize) {
    node.clone().write(|n| {
        // run one thunk
        if let Some((tt, _)) = n.thunks.front() {
            if *tt <= t {
                match n.thunks.pop_front().unwrap().1 {
                    Thunk::FetchOp(hash, from) => {
                        if let Some(op) = from.fetch_from(hash.clone()) {
                            tracing::trace!("node {:?}    fetched {:?}", n.id, hash);
                            n.vault.insert(
                                hash,
                                OpData {
                                    op,
                                    state: OpState::Pending(OpOrigin::Fetched),
                                },
                            );
                        }
                    }
                    Thunk::PublishOp(op, peer) => {
                        let hash = OpHash::from(&op);
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
            }
        }

        // handle some fetchpool item
        for _ in 0..10 {
            if let Some((op, from)) = n.fetchpool.pop_front() {
                // If no "from" specified, pick a random peer
                let peer = from.unwrap_or_else(|| {
                    n.peers[rand::thread_rng().gen_range(0, n.peers.len())].clone()
                });
                n.thunks.push_back((t + 10, Thunk::FetchOp(op, peer)));
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
                    n.fetchpool
                        .extend(deps.iter().map(|dep| (dep.clone(), None)));
                }
                OpState::Validated => {
                    op.state = OpState::Integrated;
                    // schedule publishing as soon as integrated
                    for peer in n.peers.iter() {
                        n.thunks
                            .push_back((t + 10, Thunk::PublishOp(op.op.clone(), peer.clone())));
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
        .map(Into::into)
        .take(N)
        .collect();

    // peer discovery
    for i in 0..N {
        nodes[i].write(|n| {
            n.peers.push(nodes[(i + 1) % N].clone().into());
        });
    }

    for i in 0..AUTHORED_OPS {
        nodes[0].write(|n| n.author(rand::thread_rng().gen_range(0, i + 1)));
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
