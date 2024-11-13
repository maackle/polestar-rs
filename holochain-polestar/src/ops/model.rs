// TODO: this isn't really right.
// - probably need to start with the existing list of nodes, with a None status
// - there probably needs to be a special proptest strategy for pulling from the existing list of nodes
// - this model might not even be a diagrammable state machine, maybe it needs to be further abstracted into something visually comprehensible

use std::collections::{BTreeMap, HashMap};

use anyhow::bail;
use itertools::Itertools;
use polestar::{
    diagram::montecarlo::{print_dot_state_diagram, DiagramConfig, StopCondition},
    fsm::FsmBTreeMap,
    prelude::*,
};
use proptest_derive::Arbitrary;

use super::*;

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub enum NodeOpPhase {
    #[default]
    None,
    Pending,
    Validated,
    Rejected,
    Integrated,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Arbitrary, derive_more::Display)]
pub enum NodeOpEvent {
    Author,
    Store,
    Validate,
    Reject,
    Integrate,
    Send(NodeId),
}

impl Machine for NodeOpPhase {
    type Action = NodeOpEvent;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(mut self, t: Self::Action) -> MachineResult<Self> {
        use NodeOpEvent as E;
        use NodeOpPhase as S;
        let next = match (self, t) {
            // Receive the op
            (S::None, E::Author | E::Store) => S::Pending,

            // Store is idempotent
            (s, E::Store ) => s,

            // Duplicate authorship is an error
            (s, E::Author ) => bail!("duplicate authorship"),

            (S::Pending | S::Validated, E::Validate) => S::Validated,
            (S::Pending | S::Rejected, E::Reject) => S::Rejected,

            (S::Validated, E::Integrate) => S::Integrated,
            (S::Integrated, E::Send(_)) => S::Integrated,

            (S::Integrated, _) => S::Integrated,
            (S::Rejected, _) => S::Rejected,
            (state, action) => bail!("invalid transition: {state:?} -> {action:?}"),
        };
        Ok((next, ()))
    }

    fn is_terminal(&self) -> bool {
        matches!(self, NodeOpPhase::Integrated | NodeOpPhase::Rejected)
    }
}

#[derive(Clone, Default, PartialEq, Eq, Hash, derive_more::Deref)]
pub struct NetworkOp {
    nodes: FsmBTreeMap<NodeId, NodeOpPhase>,
}

impl NetworkOp {
    pub fn new(nodes: BTreeMap<NodeId, NodeOpPhase>) -> Self {
        Self {
            nodes: FsmBTreeMap::from(nodes),
        }
    }

    pub fn new_empty(ids: &[NodeId]) -> Self {
        let mut nodes: BTreeMap<NodeId, _> = ids
            .iter()
            .map(|id| (id.clone(), Default::default()))
            .collect();
        Self {
            nodes: FsmBTreeMap::from(nodes),
        }
    }
}

impl Machine for NetworkOp {
    type Action = NetworkOpEvent;
    type Fx = ();
    type Error = String;

    fn transition(mut self, NetworkOpEvent(node_id, event): Self::Action) -> MachineResult<Self> {
        if let NodeOpEvent::Send(id) = &event {
            if !self
                .nodes
                .values()
                .any(|n| matches!(n, NodeOpPhase::Integrated))
            {
                return Err(
                    "a Send can't happen until at least one node has Integrated the op".to_string(),
                );
            }

            if node_id == *id {
                return Err("cannot send op to self".to_string());
            }

            let mut node = self.nodes.get_mut(id).unwrap();
            match node {
                NodeOpPhase::None => *node = NodeOpPhase::Pending,
                _ => return Err("don't send op twice".to_string()),
            }
        }

        if let NodeOpEvent::Author = event {
            if self.nodes.values().any(|n| !matches!(n, NodeOpPhase::None)) {
                return Err("this model only handles one Author event".to_string());
            }
        }

        self.nodes
            .transition_mut(node_id.clone(), event)
            .ok_or_else(|| format!("no node {:?}", node_id))?
            .map_err(|e| format!("{:?}", e))?;
        Ok((self, ()))
    }

    fn is_terminal(&self) -> bool {
        let vs = || self.values();

        // all integrated
        vs().all(|n| matches!(n, NodeOpPhase::Integrated))
        || (
            // all non-None are rejected
            vs().all(|n| matches!(n, NodeOpPhase::None | NodeOpPhase::Rejected) 
            // but not all None
            && vs().any(|n| !matches!(n, NodeOpPhase::None)))
        )
    }
}

impl std::fmt::Debug for NetworkOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (id, n) in self.nodes.iter() {
            writeln!(f, "{}: {:?}", id, n)?;
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Arbitrary)]
pub struct NetworkOpEvent(pub NodeId, pub NodeOpEvent);

impl std::fmt::Debug for NetworkOpEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.1 {
            NodeOpEvent::Send(id) => write!(f, "Send({}↦{})", self.0, id),
            _ => write!(f, "{:?}({})", self.1, self.0),
        }
    }
}

#[test]
#[ignore = "diagram"]
fn test_diagram() {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

    let num = 2;

    let ids = (0..num).map(|i| Id::new().into()).collect_vec();
    let (initial, ()) = NetworkOp::new_empty(&ids)
        .transition(NetworkOpEvent(ids[0].clone(), NodeOpEvent::Store))
        .unwrap();

    // TODO allow for strategy params
    print_dot_state_diagram(
        initial,
        &DiagramConfig {
            steps: 300,
            walks: 100,
            ignore_loopbacks: true,
        },
    );
}
