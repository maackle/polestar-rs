use std::collections::HashMap;

use polestar::{fsm::FsmBTreeMap, prelude::*};

use crate::ops::model::NetworkOpEvent;

use super::{model, system, NodeId, OpHash};

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
        use model::NodeOpPhase as P;
        use system::OpState as S;
        Some(model::NetworkOp::new(
            system
                .iter()
                .map(|(id, node)| {
                    let phase = node.get_op(&self.op_hash).map(|o| match o.state {
                        S::Pending(_) => P::Pending,
                        S::Validated => P::Validated,
                        S::MissingDeps(_) => todo!(),
                        S::Rejected(_) => P::Rejected,
                        S::Integrated => P::Integrated,
                    });
                    (id.clone(), phase)
                })
                .collect(),
        ))
    }

    fn map_event(&self, (id, event): Self::Event) -> Option<model::NetworkOpEvent> {
        use model::NodeOpEvent as N;
        use system::NodeEvent as E;
        use system::OpState as S;
        let n = match event {
            E::SetOpState(op, state) => match state {
                S::Rejected(_) => Some(N::Reject),
                S::Validated => Some(N::Validate),
                S::Integrated => Some(N::Integrate),
                S::Pending(op_origin) => unreachable!(),
                S::MissingDeps(vec) => unreachable!(),
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
