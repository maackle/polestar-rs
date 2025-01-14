//! A simple example of using clocks to model timeouts

#![allow(unused)]

use std::{fmt::Display, marker::PhantomData};

use crate::{
    logic::{conjoin, PropRegistry, Propositions, Transition},
    prelude::*,
    time::TimeInterval,
    util::product_exhaustive,
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

/// Action for fetch_timed
pub type Action<Agent, Val, Time> = (Agent, NodeAction<Agent, Val, Time>);

/// Action taken by a single node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, exhaustive::Exhaustive, derive_more::Display)]
pub enum NodeAction<Agent, Val, Time: TimeInterval> {
    /// Advance all clocks by a given duration
    #[display("Tick {_0}")]
    Tick(Time),

    /// Create a new value to be fetched by others
    #[display("Auth(v{_0})")]
    Author(Val),

    /// Request a value be sent
    #[display("Req(v{_0} ⇐ n{_1})")]
    Request(Val, Agent),

    /// Stop waiting on a request
    #[display("Timeout(v{_0})")]
    Timeout(Val),

    /// Receive and store a value
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

/// State for fetch_timed
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Constructor)]
pub struct State<Agent: Clone + Ord, Val: Clone + Ord, Time: Ord + Clone> {
    /// The nodes in the system
    pub nodes: OrdMap<Agent, NodeState<Val, Time>>,
}

impl<Agent: Id, Val: Id, Time: TimeInterval> Display for State<Agent, Val, Time> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}",
            self.nodes
                .iter()
                .map(|(n, s)| format!("n{n}: {s}"))
                .join("\n")
        )
    }
}

/// State for a single node
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeState<Val: Clone + Ord, Time: Clone> {
    /// The values stored by this node
    pub values: OrdSet<Val>,

    /// The outstanding requests made by this node
    pub requests: Vector<Request<Val, Time>>,
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
                .map(|r| format!("{}:{}", r.val, r.elapsed))
                .join(",")
        )
    }
}

/// A request for a value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Request<Val: Clone + Ord, Time: Clone> {
    /// The value being requested
    pub val: Val,

    /// The time elapsed since the request was made
    pub elapsed: Time,
}

impl<Val: Id, Time: TimeInterval> Request<Val, Time> {
    /// Constructor
    pub fn new(val: Val) -> Self {
        Self {
            val,
            elapsed: Time::zero(),
        }
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

/// Model for fetch_timed
pub struct Model<Agent: Id, Val: Id, Time: TimeInterval> {
    /// Interval after which a request may timeout
    pub timeout: Time,

    /// Interval after which a timeout is considered an error.
    /// Timeouts must occur between `timeout` and `timeout_grace`
    pub timeout_grace: Time,

    /// The nodes in the system
    nodes: Vec<Agent>,

    phantom: PhantomData<Val>,
}

impl<Agent: Id, Val: Id, Time: TimeInterval> Model<Agent, Val, Time> {
    /// Constructor
    pub fn new(timeout: Time, timeout_grace: Time, nodes: Vec<Agent>) -> Self {
        assert!(timeout <= timeout_grace);
        Self {
            timeout,
            timeout_grace,
            nodes,
            phantom: PhantomData,
        }
    }

    /// Initial state
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
    type Fx = ();

    fn transition(
        &self,
        mut state: Self::State,
        (node, action): Self::Action,
    ) -> TransitionResult<Self> {
        match action {
            NodeAction::Tick(dur) => {
                for req in state.nodes[&node].requests.iter_mut() {
                    if req.elapsed >= self.timeout_grace {
                        bail!(
                            "value did not time out during grace period: v={}, t={}",
                            req.val,
                            req.elapsed
                        );
                    }
                    req.elapsed = req.elapsed + dur;
                }
            }
            NodeAction::Author(v) => {
                state.nodes[&node].values.insert(v);
            }
            NodeAction::Request(v, _from) => {
                if state.nodes[&node].requests.iter().any(|r| r.val == v) {
                    bail!("request already exists")
                }

                // fixes "G (({stored} && !{req}) -> G !{req})"
                if state.nodes[&node].values.contains(&v) {
                    bail!("value already stored, don't request it again")
                }

                state.nodes[&node].requests.push_back(Request::new(v));
            }
            NodeAction::Timeout(val) => {
                let (ix, req) = state.nodes[&node]
                    .requests
                    .iter()
                    .find_position(|req| req.val == val)
                    .ok_or(anyhow!("no requests to timeout"))?;
                if req.elapsed < self.timeout {
                    bail!("timed out too early. elapsed={}", req.elapsed);
                }
                state.nodes[&node].requests.remove(ix);
            }
            NodeAction::Receive(v, found) => {
                if found {
                    state.nodes[&node].values.insert(v);
                }
                state.nodes[&node].requests.retain(|r| r.val != v);
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
                .get(agent)
                .map(|n| n.requests.iter().any(|req| req.val == *val))
                .unwrap_or(false),

            Prop::Stored(agent, val) => state
                .nodes
                .get(agent)
                .map(|n| n.values.contains(val))
                .unwrap_or(false),

            Prop::NoRequests(agent) => state.nodes[agent].requests.is_empty(),

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
        diagram::write_dot, model_checker::ModelChecker, time::FiniteTime, traversal::traverse,
    };

    const AGENTS: usize = 3;
    const VALUES: usize = 2;
    const TIMEOUT: usize = 1;
    const TIMEOUT_GRACE: usize = 1;
    const TIME_CHOICES: usize = TIMEOUT_GRACE + 1;

    type Agent = UpTo<AGENTS>;
    type Val = UpTo<VALUES>;
    type Time = FiniteTime<TIME_CHOICES, 1000>;

    type Model = crate::example_models::fetch_timed::Model<Agent, Val, Time>;

    use super::*;

    #[test]
    #[ignore = "slow"]
    fn test_fetch_timed() {
        // TODO: add system mapping stuff, from fetch_timeless
        tracing_subscriber::fmt::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        let model = Model::new(
            UpTo::new(TIMEOUT).into(),
            UpTo::new(TIMEOUT_GRACE).into(),
            (0..AGENTS).map(Agent::new).collect(),
        );
        let initial = model.initial();

        let traversal = model
            .traverse([initial])
            .ignore_loopbacks(true)
            .trace_every(100_000);

        if true
        // model check
        {
            let (props, ltl) = props_and_ltl();
            traversal.specced(props, &ltl).unwrap().model_check_report();
        } else
        // graph
        {
            let graph = traversal.diagram().unwrap();
            let graph = graph.map(|_, n| n, |_, (i, e)| format!("n{i}: {e}"));
            write_dot("out.dot", &graph, &[]);
            println!(
                "wrote out.dot. nodes={}, edges={}",
                graph.node_count(),
                graph.edge_count()
            );
        }
    }
}
