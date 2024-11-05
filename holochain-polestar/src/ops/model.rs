// TODO: this isn't really right.
// - probably need to start with the existing list of nodes, with a None status
// - there probably needs to be a special proptest strategy for pulling from the existing list of nodes
// - this model might not even be a diagrammable state machine, maybe it needs to be further abstracted into something visually comprehensible

use polestar::{actor::ActorRw, diagram::print_dot_state_diagram, Fsm};
use proptest_derive::Arbitrary;

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeOp {
    node: NodeId,
    model: NodeOpModel,
}

impl NodeOp {
    pub fn new(node: NodeId) -> Self {
        Self {
            node,
            model: NodeOpModel::Pending,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeOpModel {
    Pending,
    Validated,
    Rejected,
    Integrated,
    Sent(Vec<ActorRw<NodeOp>>),

    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Arbitrary)]
pub enum NodeOpEvent {
    Validate,
    Reject,
    Integrate,
    Send(NodeId),
}

impl Fsm for NodeOpModel {
    type Event = NodeOpEvent;
    type Fx = ();

    fn transition(&mut self, t: Self::Event) {
        use NodeOpEvent as E;
        use NodeOpModel as S;
        polestar::util::update_replace(self, |s| {
            let next = match (s, t) {
                (S::Rejected, _) => S::Rejected,
                (S::Pending, E::Validate) => S::Validated,
                (S::Pending, E::Reject) => S::Rejected,
                (S::Validated, E::Integrate) => S::Integrated,
                (S::Validated, E::Send(op)) => S::Sent(vec![NodeOp::new(op).into()]),
                (S::Sent(ops), E::Send(op)) => {
                    let mut ops = ops.clone();
                    ops.push(NodeOp::new(op).into());
                    S::Sent(ops)
                }
                _ => S::Error,
            };
            (next, ())
        });
    }
}

impl Fsm for NodeOp {
    type Event = NodeOpEvent;
    type Fx = ();

    fn transition(&mut self, t: Self::Event) {
        self.model.transition(t);
    }
}

#[test]
fn test_diagram() {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

    let node = NodeOp::new(Id::new().into());
    print_dot_state_diagram(node, 5, 10);
}
