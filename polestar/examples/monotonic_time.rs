use std::{collections::BTreeSet, sync::Arc};

use anyhow::bail;
use im::{HashMap, HashSet, OrdSet, Vector};
use itertools::Itertools;
use num_traits::Zero;
use polestar::prelude::*;
use rand::Rng;
use tokio::{sync::Mutex, time::Instant};

const NUM_VALUES: usize = 3;
const NUM_AGENTS: usize = 3;
const DELAY_CHOICES: usize = 1;

type Val = UpTo<NUM_VALUES>;
type Agent = UpTo<NUM_AGENTS>;
type Time = UpTo<DELAY_CHOICES>;
type Delay = polestar::util::Delay<Time>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Action {
    Tick,
    Author(Val),
    Request(Val, Agent),
    Timeout(Val),
    Receive(Option<Val>),
}

struct State {
    values: OrdSet<Val>,
    requests: Vector<Val>,
}

struct Model;

impl Machine for Model {
    type State = State;
    type Action = Action;
    type Error = ();

    fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        // match action {
        //     Action::Tick => {
        //         todo!()
        //     }
        //     Action::Author(v) => {
        //         todo!()
        //     }
        //     Action::Request(v) => {
        //         todo!()
        //     }
        //     Action::Receive(v) => {
        //         todo!()
        //     }
        // }
        Ok((state, ()))
    }
}

fn record_action(node: usize, action: Action) {
    match action {
        Action::Tick => println!("Tick"),
        Action::Author(val) => println!("Author v={val} by n{node}"),
        Action::Request(val, from) => println!("Request v={val} from n{from} by n{node}"),
        Action::Timeout(val) => println!("Timeout v={val} by n{node}"),
        Action::Receive(val) => println!("Receive v={val:?} by n{node}"),
    }
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct System {
    values: OrdSet<Val>,
    requests: HashMap<Val, RequestData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, derive_more::Display)]
#[display("from: {from}, time: {:?}", time.elapsed())]
struct RequestData {
    from: usize,
    time: Instant,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    const NUM_NODES: usize = 3;
    const TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(1);

    let nodes = (0..NUM_NODES)
        .map(|_| Arc::new(Mutex::new(System::default())))
        .collect::<Vec<_>>();

    for v in 0..NUM_VALUES {
        let n = v % NUM_NODES;
        let v = Val::new(v);
        let mut node = nodes[n].lock().await;
        node.values.insert(v);
        record_action(n, Action::Author(v));
    }

    loop {
        // Select target val, and sender and receiver
        let r = rand::thread_rng().gen_range(0..NUM_VALUES);
        let giver_ix = rand::thread_rng().gen_range(0..NUM_NODES);
        let mut receiver_ix = giver_ix;
        while receiver_ix == giver_ix {
            receiver_ix = rand::thread_rng().gen_range(0..NUM_NODES);
        }
        let (giver, receiver) = (nodes[giver_ix].clone(), nodes[receiver_ix].clone());
        let val = Val::new(r);

        // Make the request if not holding that value
        if !receiver.lock().await.values.contains(&val) {
            {
                let mut rcv = receiver.lock().await;
                if let Some(existing) = rcv.requests.get(&val) {
                    if existing.time.elapsed() >= TIMEOUT {
                        rcv.requests.remove(&val);
                        record_action(receiver_ix, Action::Timeout(val));
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
                record_action(receiver_ix, Action::Request(val, Agent::new(giver_ix)));
            }

            // request has 50% success rate
            if rand::thread_rng().gen_bool(0.5) {
                let delay =
                    tokio::time::Duration::from_millis(rand::thread_rng().gen_range(10..500));
                tokio::spawn(async move {
                    tokio::time::sleep(delay).await;

                    let reply = giver.lock().await.values.contains(&val).then_some(val);
                    let mut receiver = receiver.lock().await;
                    if let Some(r) = reply {
                        receiver.values.insert(r);
                    }
                    receiver.requests.remove(&val);
                    record_action(receiver_ix, Action::Receive(reply));
                });
            }
        }

        // Establish termination condition
        let mut good = true;
        // println!();
        for (i, n) in nodes.iter().enumerate() {
            let node = n.lock().await;
            // println!(
            //     "{i}: VALUES {:?} REQUESTS [{:?}]",
            //     node.values,
            //     node.requests
            //         .iter()
            //         .map(|(k, v)| format!("val={k} data={v}"))
            //         .collect_vec()
            // );
            if node.values.len() != NUM_VALUES {
                good = false;
            }
            if node.requests.len() != 0 {
                good = false;
            }
        }
        // println!();

        if good {
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
