use std::{fmt::Display, marker::PhantomData};

use crate::{
    logic::{conjoin, PropRegistry, Propositions, Transition},
    prelude::*,
    time::TimeInterval,
    util::{product2, product_exhaustive},
};
use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use im::{OrdMap, OrdSet, Vector};
use itertools::Itertools;

/*                   █████     ███
                    ░░███     ░░░
  ██████    ██████  ███████   ████   ██████  ████████
 ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
  ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
 ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
░░████████░░██████   ░░█████  █████░░██████  ████ █████
 ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░   */

pub type Action<Agent, Val, Time> = (Agent, NodeAction<Agent, Val, Time>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, exhaustive::Exhaustive, derive_more::Display)]
pub enum NodeAction<Agent, Val, Time: TimeInterval> {
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
pub struct State<Agent: Clone + Ord, Val: Clone + Ord, Time: Ord + Clone> {
    pub nodes: OrdMap<Agent, NodeState<Val, Time>>,
}

impl<Agent: Id, Val: Id, Time: TimeInterval> Display for State<Agent, Val, Time> {
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
pub struct NodeState<Val: Clone + Ord, Time: Clone> {
    pub values: OrdSet<Val>,
    pub requests: Vector<(Time, Val)>,
}

impl<Val: Id, Time: TimeInterval> Default for NodeState<Val, Time> {
    fn default() -> Self {
        Self {
            values: OrdSet::new(),
            requests: Vector::new(),
        }
    }
}

impl<Val: Id, Time: TimeInterval> Display for NodeState<Val, Time> {
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

pub struct Model<Agent: Id, Val: Id, Time: TimeInterval> {
    pub timeout: Time,
    nodes: Vec<Agent>,
    phantom: PhantomData<Val>,
}

impl<Agent: Id, Val: Id, Time: TimeInterval> Model<Agent, Val, Time> {
    pub fn new(timeout: Time, nodes: Vec<Agent>) -> Self {
        Self {
            timeout,
            nodes,
            phantom: PhantomData,
        }
    }

    pub fn initial(&self) -> State<Agent, Val, Time> {
        State {
            nodes: self
                .nodes
                .iter()
                .map(|n| (*n, NodeState::default()))
                .collect(),
        }
    }
}

impl<Agent: Id, Val: Id, Time: TimeInterval> Machine for Model<Agent, Val, Time> {
    type State = State<Agent, Val, Time>;
    type Action = Action<Agent, Val, Time>;
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
                    *time = if *time >= dur {
                        *time - dur
                    } else {
                        Time::zero()
                    };
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
            NodeAction::Timeout(val) => {
                let (ix, (time, _found)) = state.nodes[&node]
                    .requests
                    .iter()
                    .find_position(|(_, v)| *v == val)
                    .ok_or(anyhow!("no requests to timeout"))?;
                if !time.is_zero() {
                    bail!("timed out too early. time={time}")
                }
                state.nodes[&node].requests.remove(ix);
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
enum Prop<Agent: Id, Val: Id, Time: TimeInterval> {
    #[display("Requesting_n{_0}_v{_1}")]
    Requesting(Agent, Val),

    #[display("Stored_n{_0}_v{_1}")]
    Stored(Agent, Val),

    #[display("NoRequests_n{_0}")]
    NoRequests(Agent),

    #[display("Action_n{}_{}", _0.0, _0.1)]
    Action(Action<Agent, Val, Time>),
}

impl<Agent: Id, Val: Id, Time: TimeInterval> Propositions<Prop<Agent, Val, Time>>
    for Transition<Model<Agent, Val, Time>>
{
    fn eval(&self, prop: &Prop<Agent, Val, Time>) -> bool {
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

fn props_and_ltl<Agent: Id + Exhaustive, Val: Id + Exhaustive, Time: TimeInterval>(
) -> (PropRegistry<Prop<Agent, Val, Time>>, String) {
    let mut propmap = PropRegistry::empty();
    let pairs = product_exhaustive::<Agent, Val>();
    let pairwise = conjoin(pairs.flat_map(|(agent, val)| {
        let req = propmap.add(Prop::Requesting(agent, val)).unwrap();
        let stored = propmap.add(Prop::Stored(agent, val)).unwrap();

        [
            // don't make a request for data you're already storing
            format!("G (({stored} && !{req}) -> G !{req})"),
            // TODO: don't restart a round too soon
        ]
    }));
    let agentwise = conjoin(Agent::iter_exhaustive(None).flat_map(|agent| {
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
        time::FiniteTime,
        traversal::{traverse, TraversalConfig, TraversalGraphingConfig},
    };

    const AGENTS: usize = 2;
    const VALUES: usize = 2;
    const TIMEOUT: usize = 1;
    const TIME_CHOICES: usize = TIMEOUT + 1;

    type Agent = UpTo<AGENTS>;
    type Val = UpTo<VALUES>;
    type Time = FiniteTime<TIME_CHOICES, 1000>;

    type Model = crate::example_models::fetch_timed::Model<Agent, Val, Time>;

    use super::*;

    fn explore(do_graph: bool) {
        let model = Model::new(
            UpTo::<TIME_CHOICES>::new(TIMEOUT).into(),
            (0..AGENTS).map(|n| Agent::new(n)).collect(),
        );
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
        let model = Model::new(
            UpTo::<TIME_CHOICES>::new(TIMEOUT).into(),
            (0..AGENTS).map(|n| Agent::new(n)).collect(),
        );
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
