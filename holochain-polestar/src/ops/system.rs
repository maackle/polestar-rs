use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use polestar::actor::ActorRw;

use super::*;

pub struct NodeState {
    id: NodeId,
    agents: Vec<Agent>,
    vault: HashMap<OpHash, OpData>,
    cache: HashMap<OpHash, Op>,
    fetchpool: VecDeque<(OpHash, Peer)>,
    peers: Vec<Peer>,
    thunks: VecDeque<(usize, Thunk)>,
}

impl NodeState {
    pub fn new() -> Self {
        Self {
            id: Id::new().into(),
            agents: vec![],
            vault: HashMap::new(),
            cache: HashMap::new(),
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

#[derive(Clone, derive_more::From)]
pub struct Peer(Node);

pub trait NodeInterface {
    fn publish_to(&mut self, from: Peer, op: OpHash);
    fn gossip_to(&mut self, from: Peer, ops: Vec<OpHash>);
    fn fetch_from(&self, op: OpHash) -> Option<Op>;
}

impl Peer {
    fn publish_to(&self, from: Peer, op: OpHash) {
        self.0.write(|n| n.fetchpool.push_back((op, from)));
    }

    fn gossip_to(&self, from: Peer, ops: Vec<OpHash>) {
        self.0.write(|n| {
            n.fetchpool
                .extend(ops.into_iter().map(|op| (op, from.clone())))
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
    pub fn author(&mut self, op: Op) {
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
                        peer.publish_to(node.clone().into(), OpHash::from(&op));
                    }
                }
            }
        }

        // handle one fetchpool item
        if let Some((op, from)) = n.fetchpool.pop_front() {
            n.thunks.push_back((t + 10, Thunk::FetchOp(op, from)));
        }

        // move ops through the validation pipeline
        for op in n.vault.values_mut() {
            match op.state {
                OpState::Pending(_) => op.state = OpState::SysValidated,
                OpState::SysValidated => op.state = OpState::AppValidated,
                OpState::AppValidated => {
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

        // gossip ops
        if t % 10 == 0 {
            let ops: Vec<OpHash> = n
                .vault
                .values()
                .filter(|op| op.state == OpState::Integrated)
                .map(|op| OpHash::from(&op.op))
                .collect();
            for peer in n.peers.iter() {
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
    SysValidated,
    AppValidated,
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

    let ops: Vec<Op> = std::iter::repeat_with(Id::new).map(Op).take(100).collect();

    for op in ops.iter() {
        nodes[0].write(|n| n.author(op.clone()));
    }

    for t in 0..10000 {
        for n in nodes.iter() {
            step(n.clone(), t);
        }
    }

    for n in nodes.iter() {
        assert_eq!(n.read(|n| n.vault.len()), ops.len());
    }
}
