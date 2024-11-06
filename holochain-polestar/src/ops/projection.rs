use std::collections::HashMap;

use polestar::{fsm::FsmBTreeMap, prelude::*};

use crate::ops::model::NetworkOpEvent;

use super::{model, system, Id, NodeId, OpHash};

struct NetworkOpProjection {
    op_hash: OpHash,
}

impl Projection<model::NetworkOp> for NetworkOpProjection {
    type System = HashMap<NodeId, system::Node>;
    type Event = (NodeId, system::NodeEvent);

    fn apply(&self, system: &mut Self::System, (id, event): Self::Event) {
        system.get_mut(&id).unwrap().handle_event(event);
    }

    fn map_state(&self, system: &Self::System) -> Option<model::NetworkOp> {
        use model::NodeOpPhase as M;
        use system::OpState as S;
        Some(model::NetworkOp::new(
            system
                .iter()
                .map(|(id, node)| {
                    let phase = node.get_op(&self.op_hash).map(|o| match o.state {
                        S::Pending(_) => M::Pending,
                        S::Validated => M::Validated,
                        S::MissingDeps(_) => todo!(),
                        S::Rejected(_) => M::Rejected,
                        S::Integrated => M::Integrated,
                    });
                    (id.clone(), phase)
                })
                .collect(),
        ))
    }

    fn map_event(&self, (id, event): Self::Event) -> Option<model::NetworkOpEvent> {
        use model::NodeOpEvent as M;
        use system::NodeEvent as S;
        use system::OpState as O;
        let n = match event {
            S::SetOpState(op, state) => match state {
                O::Rejected(_) => Some(M::Reject),
                O::Validated => Some(M::Validate),
                O::Integrated => Some(M::Integrate),
                O::Pending(op_origin) => unreachable!(),
                O::MissingDeps(vec) => unreachable!(),
            },
            _ => None,
        }?;
        Some(NetworkOpEvent(id, n))
    }

    fn gen_state(&self, generator: &mut impl Generator, state: model::NetworkOp) -> Self::System {
        unimplemented!("generation not yet implemented for this projection")
    }

    fn gen_event(
        &self,
        generator: &mut impl Generator,
        event: model::NetworkOpEvent,
    ) -> Self::Event {
        unimplemented!("generation not yet implemented for this projection")
    }
}

#[test]
fn test_invariants() {
    let mut gen = proptest::test_runner::TestRunner::default();

    let mut system = {
        let mut nodes: HashMap<NodeId, system::Node> =
            std::iter::repeat_with(system::NodeState::new)
                .map(|s| {
                    let id: NodeId = Id::new().into();
                    (id.clone(), system::Node::new(id, s))
                })
                .take(3)
                .collect();

        system.iter_mut().next().map(|(_, n)| {
            for i in 0..5 {
                n.handle_event(system::NodeEvent::AuthorOp(0));
            }
        });
    };

    let op_hash = system[0].read(|n| n.vault.iter().next().unwrap().0.clone());
    let projection = NetworkOpProjection { op_hash };

    projection.test_invariants(
        &mut gen,
        system,
        (
            NodeId::new(),
            system::NodeEvent::SetOpState(OpHash::new(), system::OpState::Pending(NodeId::new())),
        ),
    );
}
