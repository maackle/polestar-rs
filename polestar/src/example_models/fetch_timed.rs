//! A model of

use std::{
    fmt::Display,
    marker::PhantomData,
    ops::{Mul, Sub},
};

use crate::{
    logic::{conjoin, PropRegistry, Propositions, Transition},
    prelude::*,
    time::TimeInterval,
    util::product2,
};
use anyhow::{anyhow, bail};
use im::{OrdMap, OrdSet, Vector};
use itertools::Itertools;
use num_traits::{Bounded, Zero};

pub const NUM_VALUES: usize = 1;
pub const NUM_AGENTS: usize = 2;
pub const TIMEOUT: usize = 1;
pub const TIME_CHOICES: usize = TIMEOUT + 1;

pub type Val = UpTo<NUM_VALUES>;
pub type Agent = UpTo<NUM_AGENTS>;
// pub type Time = UpTo<TIME_CHOICES>;
/*                   █████     ███
                    ░░███     ░░░
  ██████    ██████  ███████   ████   ██████  ████████
 ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
  ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
 ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
░░████████░░██████   ░░█████  █████░░██████  ████ █████
 ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░   */

pub type Action<Time> = (Agent, NodeAction<Time>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, exhaustive::Exhaustive, derive_more::Display)]
pub enum NodeAction<Time: TimeInterval> {
    #[display("Tick {_0}")]
    Tick(Time),

    #[display("Auth(v{_0})")]
    Author(Val),

    #[display("Req(v{_0} ⇐ n{_1})")]
    Request(Val, Agent),

    #[display("Timeout(v{_0})")]
    Timeout(Val),

    #[display("Recv(v{_0}, {_1})")]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Constructor)]
pub struct State<Time: TimeInterval> {
    pub nodes: OrdMap<Agent, NodeState<Time>>,
}

impl<Time: TimeInterval> Display for State<Time> {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeState<Time: TimeInterval> {
    pub values: OrdSet<Val>,
    pub requests: Vector<(Time, Val)>,
}

impl<Time: TimeInterval> Default for NodeState<Time> {
    fn default() -> Self {
        Self {
            values: OrdSet::new(),
            requests: Vector::new(),
        }
    }
}

impl<Time: TimeInterval> Display for NodeState<Time> {
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

#[derive(derive_more::Constructor)]
pub struct Model<Time: TimeInterval> {
    pub timeout: Time,
    nodes: Vec<Agent>,
}

impl<Time: TimeInterval> Model<Time> {
    pub fn initial(&self) -> State<Time> {
        State {
            nodes: self
                .nodes
                .iter()
                .map(|n| (*n, NodeState::default()))
                .collect(),
        }
    }
}

impl<Time: TimeInterval> Machine for Model<Time> {
    type State = State<Time>;
    type Action = Action<Time>;
    type Error = anyhow::Error;

    fn transition(
        &self,
        mut state: Self::State,
        (node, action): Self::Action,
    ) -> TransitionResult<Self> {
        match action {
            NodeAction::Tick(dur) => {
                for (time, v) in state.nodes[&node].requests.iter_mut() {
                    if time.is_zero() {
                        bail!("value should have timed out: v={v}")
                    }
                    *time = *time - dur;
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

                // fixes "G (({stored} && !{req}) -> G !{req})"
                if state.nodes[&node].values.contains(&v) {
                    bail!("value already stored, don't request it again")
                }

                state.nodes[&node]
                    .requests
                    .push_back((self.timeout.clone(), v));
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

/* █████       ███████████ █████
 * ░░███       ░█░░░███░░░█░░███
 *  ░███       ░   ░███  ░  ░███
 *  ░███           ░███     ░███
 *  ░███           ░███     ░███
 *  ░███      █    ░███     ░███      █
 *  ███████████    █████    ███████████
 * ░░░░░░░░░░░    ░░░░░    ░░░░░░░░░░░   */

#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
enum Prop<Time: TimeInterval> {
    #[display("Requesting_n{_0}_v{_1}")]
    Requesting(Agent, Val),

    #[display("Stored_n{_0}_v{_1}")]
    Stored(Agent, Val),

    #[display("NoRequests_n{_0}")]
    NoRequests(Agent),

    #[display("Action_n{}_{}", _0.0, _0.1)]
    Action(Action<Time>),
}

impl<Time: TimeInterval> Propositions<Prop<Time>> for Transition<Model<Time>> {
    fn eval(&self, prop: &Prop<Time>) -> bool {
        let Transition(state, action, _) = self;
        match prop {
            Prop::Requesting(agent, val) => state
                .nodes
                .get(&agent)
                .map(|n| n.requests.iter().any(|(_, v)| v == val))
                .unwrap_or(false),

            Prop::Stored(agent, val) => state
                .nodes
                .get(&agent)
                .map(|n| n.values.contains(val))
                .unwrap_or(false),

            Prop::NoRequests(agent) => state.nodes[&agent].requests.is_empty(),

            Prop::Action(a) => a == action,
        }
    }
}

fn props_and_ltl<Time: TimeInterval>() -> (PropRegistry<Prop<Time>>, String) {
    let mut propmap = PropRegistry::empty();
    let pairs = product2(Agent::all_values(), Val::all_values());
    let pairwise = conjoin(pairs.flat_map(|(agent, val)| {
        let req = propmap.add(Prop::Requesting(agent, val)).unwrap();
        let stored = propmap.add(Prop::Stored(agent, val)).unwrap();

        [
            // don't make a request for data you're already storing
            format!("G (({stored} && !{req}) -> G !{req})"),
            // TODO: don't restart a round too soon
        ]
    }));
    let agentwise = conjoin(Agent::all_values().into_iter().flat_map(|agent| {
        let no_requests = propmap.add(Prop::NoRequests(agent)).unwrap();
        [
            // always chew through all requests
            format!("G F {no_requests}"),
        ]
    }));
    let ltl = conjoin([pairwise, agentwise]);
    (propmap, ltl)
}

/*█████                      █████
 ░░███                      ░░███
 ███████    ██████   █████  ███████    █████
░░░███░    ███░░███ ███░░  ░░░███░    ███░░
  ░███    ░███████ ░░█████   ░███    ░░█████
  ░███ ███░███░░░   ░░░░███  ░███ ███ ░░░░███
  ░░█████ ░░██████  ██████   ░░█████  ██████
   ░░░░░   ░░░░░░  ░░░░░░     ░░░░░  ░░░░░░  */

#[cfg(test)]
mod tests {
    use crate::{
        diagram::write_dot,
        model_checker::{model_checker_report, ModelChecker},
        traversal::{traverse, TraversalConfig, TraversalGraphingConfig},
    };

    use super::*;

    fn explore(do_graph: bool) {
        let model = Model {
            timeout: UpTo::<TIME_CHOICES>::new(TIMEOUT),
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
        if do_graph {
            let graph = graph.unwrap();
            let graph = graph.map(|_, n| n, |_, (i, e)| format!("n{i}: {e}"));
            write_dot("out.dot", &graph, &[]);
            println!(
                "wrote out.dot. nodes={}, edges={}",
                graph.node_count(),
                graph.edge_count()
            );
        }
        dbg!(&report);
    }

    fn model_check() {
        let model = Model {
            timeout: UpTo::<TIME_CHOICES>::new(TIMEOUT),
            nodes: (0..NUM_AGENTS).map(|n| Agent::new(n)).collect(),
        };
        let (propmap, ltl) = props_and_ltl();
        println!("checking LTL:\n{}", ltl);
        let initial = model.initial();
        let checker = ModelChecker::new(model, propmap, &ltl).unwrap();

        model_checker_report(checker.check(initial));
    }

    #[test]
    fn test_fetch_timed() {
        // TODO: add system mapping stuff, from fetch_timeless
        tracing_subscriber::fmt::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        model_check();
    }
}
