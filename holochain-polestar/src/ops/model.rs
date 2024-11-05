// TODO: this isn't really right.
// - probably need to start with the existing list of nodes, with a None status
// - there probably needs to be a special proptest strategy for pulling from the existing list of nodes
// - this model might not even be a diagrammable state machine, maybe it needs to be further abstracted into something visually comprehensible

use polestar::{diagram::print_dot_state_diagram, prelude::*};
use proptest_derive::Arbitrary;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeOp {
    node: NodeId,
    model: FsmCell<NodeOpPhase>,
}

impl NodeOp {
    pub fn new(node: NodeId) -> Self {
        Self {
            node,
            model: NodeOpPhase::Pending.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeOpPhase {
    Pending,
    Validated,
    Rejected,
    Integrated,
    Sent(Vec<NodeId>),

    Error,
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
    type Error = Infallible;

    fn transition(mut self, t: Self::Event) -> FsmResult<Self> {
        use NodeOpEvent as E;
        use NodeOpPhase as S;
        let next = match (self, t) {
            (S::Rejected, _) => S::Rejected,
            (S::Pending, E::Validate) => S::Validated,
            (S::Pending, E::Reject) => S::Rejected,
            (S::Validated, E::Integrate) => S::Integrated,
            (S::Validated, E::Send(id)) => S::Sent(vec![id]),
            (S::Sent(mut ids), E::Send(id)) => {
                ids.push(id);
                S::Sent(ids)
            }
            _ => S::Error,
        };
        Ok((next, ()))
    }
}

impl Fsm for NodeOp {
    type Event = NodeOpEvent;
    type Fx = ();
    type Error = Infallible;

    fn transition(mut self, t: Self::Event) -> FsmResult<Self> {
        let () = self.model.transition_mut(t).unwrap()?;
        Ok((self, ()))
    }
}

#[test]
fn test_diagram() {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

    let node = NodeOp::new(Id::new().into());
    print_dot_state_diagram(node, 5, 10);
}
