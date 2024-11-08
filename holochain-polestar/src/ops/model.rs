// TODO: this isn't really right.
// - probably need to start with the existing list of nodes, with a None status
// - there probably needs to be a special proptest strategy for pulling from the existing list of nodes
// - this model might not even be a diagrammable state machine, maybe it needs to be further abstracted into something visually comprehensible

use std::collections::{BTreeMap, HashMap};

use itertools::Itertools;
use polestar::{
    diagram::{print_dot_state_diagram, DiagramConfig, StopCondition},
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
    Store,
    Validate,
    Reject,
    Integrate,
    Send(NodeId),
}

impl Fsm for NodeOpPhase {
    type Event = NodeOpEvent;
    type Fx = ();
    type Error = String;

    fn transition(mut self, t: Self::Event) -> FsmResult<Self> {
        use NodeOpEvent as E;
        use NodeOpPhase as S;
        let next = match (self, t) {
            (S::None, E::Store) => S::Pending,
            (S::Pending, E::Validate) => S::Validated,
            (S::Pending, E::Reject) => S::Rejected,
            (S::Validated, E::Integrate) => S::Integrated,
            (S::Integrated, E::Send(_)) => S::Integrated,

            // TODO: add these cases to fix a bug
            // (s, E::Store) => s,
            // (S::Pending | S::Validated, E::Validate) => S::Validated,
            // (S::Pending | S::Rejected, E::Reject) => S::Rejected,

            (S::Integrated, _) => S::Integrated,
            (S::Rejected, _) => S::Rejected,
            p => return Err(format!("invalid transition {:?}", p)),
        };
        Ok((next, ()))
    }

    fn is_terminal(&self) -> bool {
        matches!(self, NodeOpPhase::Integrated | NodeOpPhase::Rejected)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, derive_more::Deref)]
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
        let mut nodes: BTreeMap<NodeId, _> = ids.iter().map(|id| (id.clone(), Default::default())).collect();
        Self {
            nodes: FsmBTreeMap::from(nodes),
        }
    }
}

impl Fsm for NetworkOp {
    type Event = NetworkOpEvent;
    type Fx = ();
    type Error = Option<String>;

    fn transition(mut self, NetworkOpEvent(node_id, event): Self::Event) -> FsmResult<Self> {
        {
            let vs = || self.values();
            if
            // all integrated
            vs().all(|n| matches!(n, NodeOpPhase::Integrated))
                    || 
                    // all non-None are rejected, but not all None
                    (vs().all(|n| matches!(n, NodeOpPhase::None | NodeOpPhase::Rejected) && vs().any(|n| !matches!(n, NodeOpPhase::None))))
            {
                 // terminal states
                return Err(None)
            }
        }

        if let NodeOpEvent::Send(id) = &event {
            if node_id == *id {
                return Err(Some("cannot send op to self".to_string()));
            }
            let mut node = self.nodes.get_mut(id).unwrap();
            match node {
                NodeOpPhase::None => *node = NodeOpPhase::Pending,
                _ => return Err(Some("don't send op twice".to_string())),
            }
        }

        self.nodes
            .transition_mut(node_id.clone(), event)
            .ok_or_else(|| format!("no node {:?}", node_id))?
            .map_err(|e| format!("{:?}", e))?;
        Ok((self, ()))
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

    // print_dot_state_diagram(NodeOpPhase::default(), 5, 30);
    let ids = (0..2).map(|i| Id::new().into()).collect_vec();
    let (initial, ()) = NetworkOp::new_empty(&ids).transition(NetworkOpEvent(ids[0].clone(), NodeOpEvent::Store)).unwrap();

    // TODO allow for strategy params
    print_dot_state_diagram(initial, &DiagramConfig { steps: 1_000, walks: 300, ignore_loopbacks: true });
}
