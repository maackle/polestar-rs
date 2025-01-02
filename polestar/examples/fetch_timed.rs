//! This example demonstrates a system of nodes where each node can author a value,
//! and each other node can request that value from a node who knows about it.
//! Nodes will timeout their requests if they don't receive a response within a certain time.
//!
//! The model features a model of time, to put constraints on the behavior around timeouts, ensuring
//! that timeouts eventually happen, and that they don't happen too soon.

use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use im::{HashMap, OrdSet};
use polestar::{
    example_models::fetch_timed::{Action, Model, NodeAction, NodeState, State, *},
    mapping::{ActionOf, EventHandler, ModelMapping, StateOf},
    prelude::*,
    time::RealTime,
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

    run().await;
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

pub type Time = RealTime;

struct RealtimeMapping {
    model: Model<Time>,
    state: State<Time>,
    last_tick: Instant,
}

impl RealtimeMapping {
    pub fn new(model: Model<Time>) -> Self {
        Self {
            state: model.initial(),
            model,
            last_tick: Instant::now(),
        }
    }
}

impl polestar::mapping::ModelMapping for RealtimeMapping {
    type Model = Model<Time>;
    type System = System;
    type Event = Action<Time>;

    fn map_state(&mut self, system: &Self::System) -> Option<StateOf<Self::Model>> {
        let model = State::new(
            system
                .nodes
                .iter()
                .map(|(n, v)| {
                    (
                        Agent::new(*n),
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

    fn map_event(&mut self, event: &Self::Event) -> Vec<ActionOf<Self::Model>> {
        todo!("tick buffering");
        // let mut last_tick = Instant::now();
        // while last_tick.elapsed() >= tick_interval.into() {
        //     mapping
        //         .lock()
        //         .await
        //         .handle(&(receiver_ix, NodeAction::Tick))
        //         .unwrap();
        //     last_tick += tick_interval;
        // }

        let (node, action) = event;
        match action {
            NodeAction::Tick(RealTime) => println!("Tick n{node} {RealTime}"),
            NodeAction::Author(val) => println!("Author v={val} by n{node}"),
            NodeAction::Request(val, from) => println!("Request v={val} from n{from} by n{node}"),
            NodeAction::Timeout(val) => println!("Timeout v={val} by n{node}"),
            NodeAction::Receive(val, found) => {
                println!("Received {found} response for v={val:?} by n{node}")
            }
        }
        vec![(*node, action.clone())]
    }
}

impl polestar::mapping::EventHandler<(usize, NodeAction<Time>)> for RealtimeMapping {
    type Error = anyhow::Error;

    fn handle(&mut self, event: &(usize, NodeAction<Time>)) -> Result<(), Self::Error> {
        let event = (Agent::new(event.0), event.1);
        let actions = self.map_event(&event);
        self.state = self
            .model
            .apply_actions_(self.state.clone(), actions)
            .map_err(|(e, _, _)| e)?;
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

async fn run() {
    const NUM_NODES: usize = 3;
    let tick_interval: RealTime = RealTime::from(tokio::time::Duration::from_millis(1000));
    let timeout = tick_interval * 1;

    let nodes = (0..NUM_NODES)
        .map(|_| Arc::new(Mutex::new(SystemNode::default())))
        .collect::<Vec<_>>();
    let model = Model::new(timeout, (0..NUM_NODES).map(|n| Agent::new(n)).collect());
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
            let stop = Arc::new(AtomicBool::new(false));

            // joinset.spawn({
            //     let mapping = mapping.clone();
            //     let stop = stop.clone();
            //     async move {
            //         loop {
            //             if stop.load(std::sync::atomic::Ordering::SeqCst) {
            //                 break;
            //             }
            //             tokio::time::sleep(tick_interval).await;
            //             mapping
            //                 .lock()
            //                 .await
            //                 .handle(&(receiver_ix, NodeAction::Tick))
            //                 .unwrap();
            //         }
            //     }
            // });

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
                                if existing.time.elapsed() >= timeout.into() {
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
                        stop.store(true, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            });
        });

    joinset.join_all().await;
}
