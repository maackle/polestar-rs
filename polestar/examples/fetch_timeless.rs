//! This example demonstrates a system of nodes where each node can author a value,
//! and each other node can request that value from a node who knows about it.
//! Nodes will timeout their requests if they don't receive a response within a certain time.
//!
//! There is no limit on time. See [./monotonic_time.rs] for a version with a model of time.

use std::sync::Arc;

use anyhow::{anyhow, bail};
use im::{HashMap, OrdSet, Vector};
use polestar::{
    mapping::{ActionOf, EventHandler, ModelMapping, StateOf},
    prelude::*,
};
use rand::Rng;
use tokio::{sync::Mutex, task::JoinSet, time::Instant};

const NUM_VALUES: usize = 3;
const NUM_AGENTS: usize = 3;

type Val = UpTo<NUM_VALUES>;
type Agent = UpTo<NUM_AGENTS>;

/*                   █████     ███
                    ░░███     ░░░
  ██████    ██████  ███████   ████   ██████  ████████
 ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
  ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
 ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
░░████████░░██████   ░░█████  █████░░██████  ████ █████
 ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░   */

type Action = (Agent, NodeAction);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum NodeAction {
    // Tick,
    Author(Val),
    Request(Val, Agent),
    Timeout(Val),
    Receive(Val, bool),
}

/*        █████               █████
         ░░███               ░░███
  █████  ███████    ██████   ███████    ██████
 ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
░░█████   ░███      ███████   ░███    ░███████
 ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
 ██████   ░░█████ ░░████████  ░░█████ ░░██████
░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░  */

#[derive(Debug, Clone)]
pub struct State {
    nodes: HashMap<Agent, NodeState>,
}

#[derive(Debug, Clone, Default)]
struct NodeState {
    values: OrdSet<Val>,
    requests: Vector<Val>,
}

/*                            █████          ████
                             ░░███          ░░███
 █████████████    ██████   ███████   ██████  ░███
░░███░░███░░███  ███░░███ ███░░███  ███░░███ ░███
 ░███ ░███ ░███ ░███ ░███░███ ░███ ░███████  ░███
 ░███ ░███ ░███ ░███ ░███░███ ░███ ░███░░░   ░███
 █████░███ █████░░██████ ░░████████░░██████  █████
░░░░░ ░░░ ░░░░░  ░░░░░░   ░░░░░░░░  ░░░░░░  ░░░░░  */

struct Model {
    nodes: Vec<Agent>,
}

impl Model {
    pub fn initial(&self) -> State {
        State {
            nodes: self
                .nodes
                .iter()
                .map(|n| (*n, NodeState::default()))
                .collect(),
        }
    }
}

impl Machine for Model {
    type State = State;
    type Action = Action;
    type Error = anyhow::Error;

    fn transition(
        &self,
        mut state: Self::State,
        (node, action): Self::Action,
    ) -> TransitionResult<Self> {
        match action {
            // Action::Tick => println!("Tick"),
            NodeAction::Author(val) => println!("Author v={val} by n{node}"),
            NodeAction::Request(val, from) => println!("Request v={val} from n{from} by n{node}"),
            NodeAction::Timeout(val) => println!("Timeout v={val} by n{node}"),
            NodeAction::Receive(val, found) => {
                println!("Received {found} response for v={val:?} by n{node}")
            }
        }
        match action {
            NodeAction::Author(v) => {
                state.nodes[&node].values.insert(v);
            }
            NodeAction::Request(v, _from) => {
                state.nodes[&node].requests.push_back(v);
            }
            NodeAction::Timeout(v) => {
                let popped = state.nodes[&node]
                    .requests
                    .pop_front()
                    .ok_or(anyhow!("no requests to timeout"))?;
                if popped != v {
                    bail!("timeout doesn't match")
                }
            }
            NodeAction::Receive(v, found) => {
                if found {
                    state.nodes[&node].values.insert(v);
                }
                state.nodes[&node].requests.retain(|i| *i != v);
            }
        }

        Ok((state, ()))
    }
}

/*                                              ███
                                               ░░░
 █████████████    ██████   ████████  ████████  ████  ████████    ███████
░░███░░███░░███  ░░░░░███ ░░███░░███░░███░░███░░███ ░░███░░███  ███░░███
 ░███ ░███ ░███   ███████  ░███ ░███ ░███ ░███ ░███  ░███ ░███ ░███ ░███
 ░███ ░███ ░███  ███░░███  ░███ ░███ ░███ ░███ ░███  ░███ ░███ ░███ ░███
 █████░███ █████░░████████ ░███████  ░███████  █████ ████ █████░░███████
░░░░░ ░░░ ░░░░░  ░░░░░░░░  ░███░░░   ░███░░░  ░░░░░ ░░░░ ░░░░░  ░░░░░███
                           ░███      ░███                       ███ ░███
                           █████     █████                     ░░██████
                          ░░░░░     ░░░░░                       ░░░░░░   */

struct RealtimeMapping {
    model: Model,
    state: State,
}

impl RealtimeMapping {
    pub fn new(model: Model) -> Self {
        Self {
            state: model.initial(),
            model,
        }
    }
}

impl polestar::mapping::ModelMapping for RealtimeMapping {
    type Model = Model;
    type System = System;
    type Event = Action;

    fn map_state(&mut self, system: &Self::System) -> Option<StateOf<Self::Model>> {
        let model = State {
            nodes: system
                .nodes
                .iter()
                .map(|(n, v)| {
                    (
                        Agent::new(*n),
                        NodeState {
                            values: v.values.clone(),
                            requests: v.requests.iter().map(|(v, _)| v.clone()).collect(),
                        },
                    )
                })
                .collect(),
        };
        Some(model)
    }

    fn map_event(&mut self, event: &Self::Event) -> Option<ActionOf<Self::Model>> {
        Some((event.0, event.1.clone()))
    }
}

impl polestar::mapping::EventHandler<(usize, NodeAction)> for RealtimeMapping {
    type Error = anyhow::Error;

    fn handle(&mut self, event: &(usize, NodeAction)) -> Result<(), Self::Error> {
        let event = (Agent::new(event.0), event.1);
        let action = self.map_event(&event).unwrap();
        self.state = self.model.transition_(self.state.clone(), action)?;
        Ok(())
    }
}

/*                           █████
                            ░░███
  █████  █████ ████  █████  ███████    ██████  █████████████
 ███░░  ░░███ ░███  ███░░  ░░░███░    ███░░███░░███░░███░░███
░░█████  ░███ ░███ ░░█████   ░███    ░███████  ░███ ░███ ░███
 ░░░░███ ░███ ░███  ░░░░███  ░███ ███░███░░░   ░███ ░███ ░███
 ██████  ░░███████  ██████   ░░█████ ░░██████  █████░███ █████
░░░░░░    ░░░░░███ ░░░░░░     ░░░░░   ░░░░░░  ░░░░░ ░░░ ░░░░░
          ███ ░███
         ░░██████
          ░░░░░░                                              */

struct System {
    nodes: HashMap<usize, SystemNode>,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SystemNode {
    values: OrdSet<Val>,
    requests: HashMap<Val, RequestData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, derive_more::Display)]
#[display("from: {from}, time: {:?}", time.elapsed())]
struct RequestData {
    from: usize,
    time: Instant,
}

/*                          ███
                           ░░░
 █████████████    ██████   ████  ████████
░░███░░███░░███  ░░░░░███ ░░███ ░░███░░███
 ░███ ░███ ░███   ███████  ░███  ░███ ░███
 ░███ ░███ ░███  ███░░███  ░███  ░███ ░███
 █████░███ █████░░████████ █████ ████ █████
░░░░░ ░░░ ░░░░░  ░░░░░░░░ ░░░░░ ░░░░ ░░░░░   */

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    const NUM_NODES: usize = 3;
    const TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(1);

    let nodes = (0..NUM_NODES)
        .map(|_| Arc::new(Mutex::new(SystemNode::default())))
        .collect::<Vec<_>>();
    let model = Model {
        nodes: (0..NUM_NODES).map(|n| Agent::new(n)).collect(),
    };
    let mapping = Arc::new(Mutex::new(RealtimeMapping::new(model)));

    for v in 0..NUM_VALUES {
        let n = v % NUM_NODES;
        let v = Val::new(v);
        let mut node = nodes[n].lock().await;
        node.values.insert(v);
        mapping
            .lock()
            .await
            .handle(&(n, NodeAction::Author(v)))
            .unwrap();
    }

    let mut joinset = JoinSet::new();

    nodes
        .clone()
        .into_iter()
        .enumerate()
        .for_each(|(receiver_ix, receiver)| {
            let nodes = nodes.clone();
            let mapping = mapping.clone();
            joinset.spawn(async move {
                loop {
                    let receiver = receiver.clone();
                    // Select target val and requestee
                    let r = rand::thread_rng().gen_range(0..NUM_VALUES);
                    let mut giver_ix = rand::thread_rng().gen_range(0..NUM_NODES);
                    while receiver_ix == giver_ix {
                        giver_ix = rand::thread_rng().gen_range(0..NUM_NODES);
                    }
                    let giver = nodes[giver_ix].clone();
                    let val = Val::new(r);

                    // Make the request if not holding that value
                    if !receiver.lock().await.values.contains(&val) {
                        {
                            let mut rcv = receiver.lock().await;
                            if let Some(existing) = rcv.requests.get(&val) {
                                if existing.time.elapsed() >= TIMEOUT {
                                    rcv.requests.remove(&val);
                                    mapping
                                        .lock()
                                        .await
                                        .handle(&(receiver_ix, NodeAction::Timeout(val)))
                                        .unwrap();
                                } else {
                                    continue;
                                }
                            }
                            rcv.requests.insert(
                                val,
                                RequestData {
                                    from: giver_ix,
                                    time: Instant::now(),
                                },
                            );
                            mapping
                                .lock()
                                .await
                                .handle(&(
                                    receiver_ix,
                                    NodeAction::Request(val, Agent::new(giver_ix)),
                                ))
                                .unwrap();
                        }

                        // request has 50% success rate
                        if rand::thread_rng().gen_bool(0.5) {
                            let delay = tokio::time::Duration::from_millis(
                                rand::thread_rng().gen_range(10..500),
                            );
                            let receiver = receiver.clone();
                            let mapping = mapping.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(delay).await;

                                let reply = giver.lock().await.values.contains(&val).then_some(val);
                                let mut receiver = receiver.lock().await;
                                if let Some(r) = reply {
                                    receiver.values.insert(r);
                                }
                                receiver.requests.remove(&val);
                                mapping
                                    .lock()
                                    .await
                                    .handle(&(
                                        receiver_ix,
                                        NodeAction::Receive(val, reply.is_some()),
                                    ))
                                    .unwrap();
                            });
                        }
                    }

                    // Establish termination condition
                    let mut good = true;
                    if receiver.lock().await.values.len() != NUM_VALUES {
                        good = false;
                    }
                    if receiver.lock().await.requests.len() != 0 {
                        good = false;
                    }

                    if good {
                        break;
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            });
        });

    joinset.join_all().await;
}
