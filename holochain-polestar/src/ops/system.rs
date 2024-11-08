use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fmt::Debug,
    sync::{mpsc, Arc},
};

use polestar::actor::ActorRw;
use rand::{seq::IteratorRandom, Rng};

use super::*;

#[derive(Clone, Debug)]
pub struct NodeState {
    // pub agents: Vec<Agent>,
    pub vault: BTreeMap<OpHash, OpData>,
    pub cache: HashMap<OpHash, Op>,
}

#[derive(Debug)]
pub struct NodeConnections {
    pub sender: mpsc::Sender<(NodeId, Message)>,
    pub inbox: mpsc::Receiver<(NodeId, Message)>,
    pub outboxes: HashMap<NodeId, mpsc::Sender<(NodeId, Message)>>,
}

impl NodeConnections {
    pub fn new() -> Self {
        let (sender, inbox) = mpsc::channel();
        Self {
            sender,
            inbox,
            outboxes: HashMap::new(),
        }
    }
}

impl NodeState {
    pub fn new() -> Self {
        Self {
            // agents: vec![],
            vault: Default::default(),
            cache: Default::default(),
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
    // AddPeer(NodeId),
    StoreOp(Op, FetchDestination),
    SetOpState(OpHash, OpState),

    SendOp(Op, NodeId),
}

#[derive(Clone, Debug)]
pub enum FetchDestination {
    Vault,
    Cache,
}

/// A node in the network
#[derive(derive_more::From, derive_more::Deref)]
pub struct Node {
    id: NodeId,
    #[deref]
    state: ActorRw<NodeState>,
    connections: NodeConnections,
    tee: mpsc::Sender<(NodeId, NodeEvent)>,
}

impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("state", &self.state)
            .finish()
    }
}

impl Node {
    pub fn new(id: NodeId, state: NodeState, tee: mpsc::Sender<(NodeId, NodeEvent)>) -> Self {
        Self {
            id,
            state: ActorRw::new(state),
            connections: NodeConnections::new(),
            tee,
        }
    }

    #[tracing::instrument(skip(self), fields(id = %self.id))]
    pub fn handle_event(&mut self, event: NodeEvent) {
        self.tee.send((self.id.clone(), event.clone())).unwrap();
        if let NodeEvent::SendOp(op, peer) = &event {
            self.send(peer, Message::Publish(op.clone()));
        }
        self.state.write(|n| n.handle_event(event));
    }

    pub fn handle_message(&mut self, (from, msg): (NodeId, Message)) {
        match msg {
            Message::Publish(op) => {
                self.handle_event(NodeEvent::StoreOp(op, FetchDestination::Vault))
            }
            Message::Gossip(ops) => {
                for op in ops {
                    self.handle_event(NodeEvent::StoreOp(op, FetchDestination::Vault))
                }
            }
            Message::FetchRequest(hash) => {
                if let Some(op) = self
                    .get_op(&hash)
                    .filter(|op| op.state == OpState::Integrated)
                {
                    self.send(&from, Message::FetchResponse(op.op));
                }
            }
            Message::FetchResponse(op) => {
                self.handle_event(NodeEvent::StoreOp(op, FetchDestination::Cache))
            }
        }
    }

    pub fn process_inbox(&mut self) {
        if let Ok(msg) = self.connections.inbox.try_recv() {
            self.handle_message(msg);
        }
    }

    pub fn send(&self, peer: &NodeId, msg: Message) {
        self.connections.outboxes[peer]
            .send((self.id.clone(), msg))
            .unwrap();
    }

    pub fn send_random(&self, msg: Message) {
        self.connections
            .outboxes
            .values()
            .choose(&mut rand::thread_rng())
            .unwrap()
            .send((self.id.clone(), msg))
            .unwrap();
    }

    pub fn add_connection(&mut self, peer: NodeId, tx: mpsc::Sender<(NodeId, Message)>) {
        self.connections.outboxes.insert(peer, tx);
    }

    pub fn get_sender(&self) -> mpsc::Sender<(NodeId, Message)> {
        self.connections.sender.clone()
    }

    pub fn get_op(&self, hash: &OpHash) -> Option<OpData> {
        self.read(|n| n.vault.get(hash).cloned())
    }
}

impl NodeState {
    pub fn handle_event(&mut self, event: NodeEvent) {
        match event {
            NodeEvent::AuthorOp(num_deps) => self.author(num_deps),
            NodeEvent::StoreOp(op, destination) => {
                self.store(op, destination);
            }
            NodeEvent::SetOpState(hash, state) => {
                if let Some(op) = self.vault.get_mut(&hash) {
                    use OpState::*;
                    let valid = matches!(
                        (&op.state, &state),
                        (Pending(_), Validated)
                            | (Pending(_), Rejected(_))
                            | (Validated, Integrated)
                            | (MissingDeps(_), Validated)
                    );
                    if valid {
                        op.state = state
                    };
                };
            }
            NodeEvent::SendOp(op, peer) => {
                // not handled in the real system
            }
        }
    }

    pub fn get_op(&self, hash: &OpHash) -> Option<OpData> {
        self.vault.get(hash).cloned()
    }

    #[tracing::instrument(skip(self))]
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

    #[tracing::instrument(skip(self))]
    fn store(&mut self, op: Op, destination: FetchDestination) {
        tracing::info!("stored op");
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

pub enum Message {
    Publish(Op),
    Gossip(Vec<Op>),
    FetchRequest(OpHash),
    FetchResponse(Op),
}

// fn fetch_from(&self, op: OpHash) -> Option<Op> {
//     self.0.read(|n| {
//         n.vault
//             .get(&op)
//             .filter(|op_data| op_data.state == OpState::Integrated)
//             .map(|op_data| op_data.op.clone())
//     })
// }

pub fn step(node: &mut Node, t: usize) {
    // move ops through the validation pipeline
    let mut to_validate: Vec<Op> = vec![];
    let mut events = vec![];
    let mut to_publish = vec![];
    node.read(|n| {
        for op in n.vault.values() {
            match &op.state {
                OpState::Pending(_) => {
                    to_validate.push(op.op.clone());
                }
                OpState::MissingDeps(deps) => {
                    todo!()
                    // events.extend(deps.iter().map(|dep| {
                    //     NodeEvent::EnqueueFetch(dep.clone(), None, FetchDestination::Cache)
                    // }));
                }
                OpState::Validated => {
                    events.push(NodeEvent::SetOpState(
                        op.op.hash.clone(),
                        OpState::Integrated,
                    ));

                    to_publish.push(op.op.clone());
                }
                _ => {}
            }
        }
    });

    for op in to_publish {
        for tx in node.connections.outboxes.values() {
            tx.send((node.id.clone(), Message::Publish(op.clone())))
                .unwrap();
        }
    }

    for e in events.drain(..) {
        node.handle_event(e);
    }

    node.read(|n| {
        for op in to_validate {
            if op
                .deps
                .iter()
                .all(|dep| n.vault.contains_key(dep) || n.cache.contains_key(dep))
            {
                events.push(NodeEvent::SetOpState(op.hash.clone(), OpState::Validated));
            } else {
                events.push(NodeEvent::SetOpState(
                    op.hash.clone(),
                    OpState::MissingDeps(op.deps),
                ));
            }
        }
    });

    for e in events.drain(..) {
        node.handle_event(e);
    }

    // gossip ops
    if t % 10 == 0 {
        let ops: Vec<Op> = node.read(|n| {
            n.vault
                .values()
                .filter(|op| op.state == OpState::Integrated)
                .map(|op| op.op.clone())
                .collect()
        });
        node.send_random(Message::Gossip(ops));
    }

    for e in events.drain(..) {
        node.handle_event(e);
    }
}

#[derive(Clone, Debug)]
pub struct OpData {
    pub op: Op,
    pub state: OpState,
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

    use polestar::{prelude::ProjectionDown, Fsm};
    use projection::NetworkOpProjection;
    use rand::Rng;
    use system::{Node, NodeEvent, NodeState};

    use super::*;

    #[test]
    fn test_node() {
        tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

        const N: usize = 3;
        const AUTHORED_OPS: usize = 10;
        const MAX_ITERS: usize = 100_000;

        let (event_tx, event_rx) = mpsc::channel();

        let initial: HashMap<NodeId, NodeState> =
            std::iter::repeat_with(|| (Id::new().into(), NodeState::new()))
                .take(N)
                .collect();

        let mut nodes: Vec<Node> = initial
            .iter()
            .map(|(id, s)| Node::new(id.clone(), s.clone(), event_tx.clone()))
            .collect();

        // peer discovery
        for i in 0..N {
            let id = nodes[(i + 1) % N].id.clone();
            let sender = nodes[(i + 1) % N].connections.sender.clone();
            nodes[i].add_connection(id, sender);
        }

        for i in 0..AUTHORED_OPS {
            nodes[0].handle_event(NodeEvent::AuthorOp(rand::thread_rng().gen_range(0..i + 1)));
        }

        for t in 0..MAX_ITERS {
            println!("t = {t}");

            for n in nodes.iter_mut() {
                step(n, t);
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

        {
            let op = nodes[0].read(|n| n.vault.values().next().unwrap().op.clone());
            let projection = NetworkOpProjection { op };
            let mut model = projection.map_state(&initial).unwrap();

            while let Ok(event) = event_rx.try_recv() {
                if let Some(action) = projection.map_event(event) {
                    model = model.transition_(action).unwrap()
                } else {
                    tracing::info!("no event mapped");
                }
            }
        }

        for n in nodes.iter() {
            assert_eq!(
                n.read(|n| n.num_integrated()),
                AUTHORED_OPS,
                "node {} has {} integrated ops",
                n.id,
                n.read(|n| n.num_integrated())
            );
        }
    }
}
