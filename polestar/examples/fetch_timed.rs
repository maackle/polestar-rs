//! This example demonstrates a system of nodes where each node can author a value,
//! and each other node can request that value from a node who knows about it.
//! Nodes will timeout their requests if they don't receive a response within a certain time.
//!
//! The model features a model of time, to put constraints on the behavior around timeouts, ensuring
//! that timeouts eventually happen, and that they don't happen too soon.
//!
//! KNOWN ISSUES:
//! - there is a race condition where a node can request a value even after it has stored that value

use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use im::{HashMap, OrdSet};
use itertools::Itertools;
use polestar::{
    example_models::fetch_timed::{Action, Model, NodeAction, NodeState, State, *},
    mapping::{ActionOf, EventHandler, ModelMapping, StateOf},
    prelude::*,
    time::{RealTime, TickBuffer},
};
use rand::Rng;
use tokio::{sync::Mutex, task::JoinSet, time::Instant};

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
    // TODO: add system mapping stuff, from fetch_timeless
    tracing_subscriber::fmt::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    run(7, 11).await;
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

pub type Agent = usize;
pub type Val = usize;
pub type Time = RealTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    Author(Val),
    Request(Val, Agent),
    Timeout(Val),
    Receive(Val, bool),
}

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

async fn run(num_agents: usize, num_values: usize) {
    let timeout = RealTime::from(tokio::time::Duration::from_millis(1000));

    let nodes = (0..num_agents)
        .map(|_| Arc::new(Mutex::new(SystemNode::default())))
        .collect::<Vec<_>>();
    let model = Model::new(
        timeout,
        (0..num_agents)
            .map(|n| Agent::try_from(n).unwrap())
            .collect(),
    );
    let mapping = Arc::new(Mutex::new(RealtimeMapping::new(model, Instant::now())));

    for v in 0..num_values {
        let n = v % num_agents;
        let v = Val::from(v);
        let mut node = nodes[n].lock().await;
        node.values.insert(v);
        mapping.lock().await.handle(&(n, Event::Author(v))).unwrap();
    }

    let mut joinset = JoinSet::new();

    nodes
        .clone()
        .into_iter()
        .enumerate()
        .for_each(|(receiver_ix, receiver)| {
            let nodes = nodes.clone();
            let mapping = mapping.clone();
            let stop = Arc::new(AtomicBool::new(false));

            joinset.spawn(async move {
                loop {
                    let receiver = receiver.clone();
                    // Select target val and requestee
                    let r = rand::thread_rng().gen_range(0..num_values);
                    let mut giver_ix = rand::thread_rng().gen_range(0..num_agents);
                    while receiver_ix == giver_ix {
                        giver_ix = rand::thread_rng().gen_range(0..num_agents);
                    }
                    let giver = nodes[giver_ix].clone();
                    let val = r;

                    // Make the request if not holding that value
                    if !receiver.lock().await.values.contains(&val) {
                        {
                            let mut rcv = receiver.lock().await;
                            if let Some(existing) = rcv.requests.get(&val) {
                                if existing.time.elapsed() >= timeout.into() {
                                    rcv.requests.remove(&val);
                                    mapping
                                        .lock()
                                        .await
                                        .handle(&(receiver_ix, Event::Timeout(val)))
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
                                .handle(&(receiver_ix, Event::Request(val, Agent::from(giver_ix))))
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
                                    .handle(&(receiver_ix, Event::Receive(val, reply.is_some())))
                                    .unwrap();
                            });
                        }
                    }

                    // Establish termination condition
                    let mut good = true;
                    if receiver.lock().await.values.len() != num_values {
                        good = false;
                    }
                    if receiver.lock().await.requests.len() != 0 {
                        good = false;
                    }

                    if good {
                        stop.store(true, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            });
        });

    joinset.join_all().await;
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
    model: Model<Agent, Val, Time>,
    state: State<Agent, Val, Time>,
    actions: Vec<Action<Agent, Val, Time>>,
    start_time: Instant,
    tick_buffers: Vec<TickBuffer<Time>>,
}

impl RealtimeMapping {
    pub fn new(model: Model<Agent, Val, Time>, start_time: Instant) -> Self {
        Self {
            actions: vec![],
            state: model.initial(),
            model,
            tick_buffers: vec![],
            start_time,
        }
    }
}

impl polestar::mapping::ModelMapping for RealtimeMapping {
    type Model = Model<Agent, Val, Time>;
    type System = System;
    type Event = (Agent, Event);

    fn map_state(&mut self, system: &Self::System) -> Option<StateOf<Self::Model>> {
        let model = State::new(
            system
                .nodes
                .iter()
                .map(|(n, v)| {
                    (
                        Agent::try_from(*n).unwrap(),
                        NodeState {
                            values: v.values.clone(),
                            requests: v
                                .requests
                                .iter()
                                .map(|(v, _)| (self.model.timeout, v.clone()))
                                .collect(),
                        },
                    )
                })
                .collect(),
        );
        Some(model)
    }

    fn map_event(&mut self, (node, event): &Self::Event) -> Vec<ActionOf<Self::Model>> {
        if *node >= self.tick_buffers.len() {
            self.tick_buffers
                .resize_with(node + 1, || TickBuffer::new(self.start_time.into()));
        }
        let mut actions = self.tick_buffers[*node]
            .tick(Instant::now().into())
            .map(|t| {
                println!("tick {node} {t}");
                (*node, NodeAction::Tick(t))
            })
            .collect_vec();

        let action = match event {
            Event::Author(val) => {
                println!("Author v={val} by n{node}");
                NodeAction::Author(*val)
            }
            Event::Request(val, from) => {
                println!("Request v={val} from n{from} by n{node}");
                NodeAction::Request(*val, *from)
            }
            Event::Timeout(val) => {
                println!("Timeout v={val} by n{node}");
                NodeAction::Timeout(*val)
            }
            Event::Receive(val, found) => {
                println!("Received {found} response for v={val:?} by n{node}");
                NodeAction::Receive(*val, *found)
            }
        };

        actions.push((*node, action.clone()));

        self.actions.extend(actions.clone());

        actions
    }
}

impl polestar::mapping::EventHandler<(usize, Event)> for RealtimeMapping {
    type Error = anyhow::Error;

    fn handle(&mut self, event: &(usize, Event)) -> Result<(), Self::Error> {
        let event = (Agent::from(event.0), event.1);
        let actions = self.map_event(&event);
        self.state = self
            .model
            .apply_actions_(self.state.clone(), actions)
            .map_err(|(e, s, a)| {
                println!("MAPPING ERROR.");
                println!();
                println!("All actions:");
                for (i, action) in self.actions.iter().enumerate() {
                    println!("{i:>4}: {action:?}");
                }
                println!();
                println!("Last state: {s:?}");
                println!();

                if a != *self.actions.last().unwrap() {
                    println!("WARNING: Last action different (what does this mean?): {a:?}");
                }
                e
            })
            .unwrap();
        Ok(())
    }
}
