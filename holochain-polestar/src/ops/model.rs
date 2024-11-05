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
    model: CellFsm<NodeOpPhase>,
}

impl NodeOp {
    pub fn new(node: NodeId) -> Self {
        Self {
            node,
            model: NodeOpPhase::Pending.into(),
        }
    }
}

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
    Send,
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
            (S::Integrated, E::Send) => S::Integrated,

            (S::Rejected, _) => return Err("cannot transition rejected op".to_string()),
            _ => return Err("invalid transition".to_string()),
        };
        Ok((next, ()))
    }
}

impl Fsm for NodeOp {
    type Event = NodeOpEvent;
    type Fx = ();
    type Error = String;

    fn transition(mut self, t: Self::Event) -> FsmResult<Self> {
        let () = self.model.transition_mut(t).unwrap()?;
        Ok((self, ()))
    }
}

#[test]
fn test_diagram() {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

    print_dot_state_diagram(NodeOpPhase::default(), 5, 30);
}
