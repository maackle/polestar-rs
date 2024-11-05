// TODO: this isn't really right.
// - probably need to start with the existing list of nodes, with a None status
// - there probably needs to be a special proptest strategy for pulling from the existing list of nodes
// - this model might not even be a diagrammable state machine, maybe it needs to be further abstracted into something visually comprehensible

use std::collections::{BTreeMap, HashMap};

use polestar::{diagram::print_dot_state_diagram, fsm::FsmBTreeMap, prelude::*};
use proptest_derive::Arbitrary;

use super::*;

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub enum NodeOpPhase {
    #[default]
    Pending,
    Validated,
    Rejected,
    Integrated,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Arbitrary)]
pub enum NodeOpEvent {
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
            (S::Pending, E::Validate) => S::Validated,
            (S::Pending, E::Reject) => S::Rejected,
            (S::Validated, E::Integrate) => S::Integrated,
            (S::Integrated, E::Send(_)) => S::Integrated,

            (S::Rejected, _) => return Err("cannot transition rejected op".to_string()),
            _ => return Err("invalid transition".to_string()),
        };
        Ok((next, ()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkOp {
    nodes: FsmBTreeMap<NodeId, Option<NodeOpPhase>>,
}

impl NetworkOp {
    pub fn new_single_op(num: usize) -> Self {
        let mut nodes: BTreeMap<NodeId, _> = (0..num).map(|_| (Id::new().into(), None)).collect();
        *nodes.iter_mut().next().unwrap().1 = Some(NodeOpPhase::Pending);
        Self {
            nodes: FsmBTreeMap::from(nodes),
        }
    }
}

impl Fsm for NetworkOp {
    type Event = (NodeId, NodeOpEvent);
    type Fx = ();
    type Error = String;

    fn transition(mut self, (node, event): Self::Event) -> FsmResult<Self> {
        if let NodeOpEvent::Send(id) = &event {
            let mut node = self.nodes.get_mut(id).unwrap();
            if node.is_none() {
                *node = Some(NodeOpPhase::Pending);
            }
        }
        self.nodes
            .transition_mut(node.clone(), event)
            .ok_or_else(|| format!("no node {:?}", node))?
            .map_err(|e| format!("{:?}", e))?;
        Ok((self, ()))
    }
}

#[test]
fn test_diagram() {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

    print_dot_state_diagram(NodeOpPhase::default(), 5, 30);

    print_dot_state_diagram(NetworkOp::new_single_op(2), 5000, 30);
}
