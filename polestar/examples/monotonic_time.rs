//! This example demonstrates a system of nodes where each node can author a value,
//! and each other node can request that value from a node who knows about it.
//! Nodes will timeout their requests if they don't receive a response within a certain time.
//!
//! The model features a model of time, to put constraints on the behavior around timeouts, ensuring
//! that timeouts eventually happen, and that they don't happen too soon.

use std::{fmt::Display, sync::Arc};

use anyhow::{anyhow, bail};
use im::{HashMap, OrdMap, OrdSet, Vector};
use itertools::Itertools;
use num_traits::Zero;
use polestar::{
    diagram::write_dot,
    mapping::{ActionOf, EventHandler, ModelMapping, StateOf},
    prelude::*,
    traversal::{traverse, TraversalConfig, TraversalGraphingConfig},
};
use rand::Rng;
use tokio::{sync::Mutex, task::JoinSet, time::Instant};

const NUM_VALUES: usize = 2;
const NUM_AGENTS: usize = 3;
const TIMEOUT: usize = 1;
const TIME_CHOICES: usize = TIMEOUT + 1;

type Val = UpTo<NUM_VALUES>;
type Agent = UpTo<NUM_AGENTS>;
type Time = UpTo<TIME_CHOICES>;
type Delay = polestar::util::Delay<Time>;

/*                   █████     ███
                    ░░███     ░░░
  ██████    ██████  ███████   ████   ██████  ████████
 ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
  ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
 ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
░░████████░░██████   ░░█████  █████░░██████  ████ █████
 ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░   */

pub type Action = (Agent, NodeAction);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, exhaustive::Exhaustive, derive_more::Display)]
enum NodeAction {
    Tick,
    #[display("Author(v{_0})")]
    Author(Val),
    #[display("Request(v{_0} <- n{_1})")]
    Request(Val, Agent),
    #[display("Timeout(v{_0})")]
    Timeout(Val),
    #[display("Receive(v{_0}, {_1})")]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    nodes: OrdMap<Agent, NodeState>,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n",
            self.nodes
                .iter()
                .map(|(n, s)| format!("n{n}: {s}"))
                .join("\n")
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct NodeState {
    values: OrdSet<Val>,
    requests: Vector<(Time, Val)>,
}

impl Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stored [{}] requests [{}]",
            self.values.iter().join(" "),
            self.requests
                .iter()
                .map(|(v, t)| format!("{v}:{t}"))
                .join(",")
        )
    }
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
        // match action {
        //     NodeAction::Tick => println!("Tick"),
        //     NodeAction::Author(val) => println!("Author v={val} by n{node}"),
        //     NodeAction::Request(val, from) => println!("Request v={val} from n{from} by n{node}"),
        //     NodeAction::Timeout(val) => println!("Timeout v={val} by n{node}"),
        //     NodeAction::Receive(val, found) => {
        //         println!("Received {found} response for v={val:?} by n{node}")
        //     }
        // }
        match action {
            NodeAction::Tick => {
                for (time, v) in state.nodes[&node].requests.iter_mut() {
                    if time.is_zero() {
                        bail!("value should have timed out: v={v}")
                    }
                    *time = *time - 1;
                }
            }
            NodeAction::Author(v) => {
                state.nodes[&node].values.insert(v);
            }
            NodeAction::Request(v, _from) => {
                if state.nodes[&node]
                    .requests
                    .iter()
                    .find(|(_, i)| *i == v)
                    .is_some()
                {
                    bail!("request already exists")
                }
                state.nodes[&node]
                    .requests
                    .push_back((Time::new(TIMEOUT), v));
            }
            NodeAction::Timeout(v) => {
                let (time, popped) = state.nodes[&node]
                    .requests
                    .pop_front()
                    .ok_or(anyhow!("no requests to timeout"))?;
                if popped != v {
                    bail!("timeout doesn't match")
                }
                if !time.is_zero() {
                    bail!("timed out too early or too late")
                }
            }
            NodeAction::Receive(v, found) => {
                if found {
                    state.nodes[&node].values.insert(v);
                }
                state.nodes[&node].requests.retain(|(_, i)| *i != v);
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
        todo!();
        // let model = State {
        //     nodes: system
        //         .nodes
        //         .iter()
        //         .map(|(n, v)| {
        //             (
        //                 Agent::new(*n),
        //                 NodeState {
        //                     values: v.values.clone(),
        //                     requests: v.requests.iter().map(|(v, _)| v.clone()).collect(),
        //                 },
        //             )
        //         })
        //         .collect(),
        // };
        // Some(model)
    }

    fn map_event(&mut self, event: &Self::Event) -> Option<ActionOf<Self::Model>> {
        // match event.1 {
        //     NodeAction::Tick => {
        //         self.recent_ticks.push_front(Instant::now());
        //     }
        //     _ => {}
        // }
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

fn explore(do_graph: bool) {
    let model = Model {
        nodes: (0..NUM_AGENTS).map(|n| Agent::new(n)).collect(),
    };
    let initial = model.initial();
    let config = TraversalConfig::builder()
        .graphing(TraversalGraphingConfig {
            ignore_loopbacks: true,
        })
        .trace_every(100_000)
        // .trace_error(true)
        .build();

    let (report, graph, _) = traverse(model.into(), initial, config, Some).unwrap();
    dbg!(&report);

    if do_graph {
        let graph = graph.unwrap();
        let graph = graph.map(|_, n| n, |_, (i, e)| format!("n{i}: {e}"));
        write_dot("out.dot", &graph, &[]);
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    explore(false);
    return;

    const NUM_NODES: usize = NUM_AGENTS;
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
