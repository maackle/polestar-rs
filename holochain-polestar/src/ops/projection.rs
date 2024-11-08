use std::collections::HashMap;

use polestar::{fsm::FsmBTreeMap, prelude::*};

use crate::ops::model::NetworkOpEvent;

use super::{model, system, Id, NodeId, Op, OpHash};

struct NetworkOpProjection {
    op: Op,
}

impl ProjectionDown<model::NetworkOp> for NetworkOpProjection {
    type System = HashMap<NodeId, system::NodeState>;
    type Event = (NodeId, system::NodeEvent);

    fn apply(&self, system: &mut Self::System, (id, event): Self::Event) {
        if let Some(node) = system.get_mut(&id) {
            node.handle_event(event);
        }
    }

    fn map_state(&self, system: &Self::System) -> Option<model::NetworkOp> {
        use model::NodeOpPhase as M;
        use system::OpState as S;
        Some(model::NetworkOp::new(
            system
                .iter()
                .map(|(id, node)| {
                    let phase = node
                        .get_op(&self.op.hash)
                        .map(|o| match o.state {
                            S::Pending(_) => M::Pending,
                            S::Validated => M::Validated,
                            S::MissingDeps(_) => todo!(),
                            S::Rejected(_) => M::Rejected,
                            S::Integrated => M::Integrated,
                        })
                        .unwrap_or(M::None);
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
            S::AuthorOp(0) => todo!(),
            S::StoreOp(op, system::FetchDestination::Vault) => todo!(),
            S::SendOp(op, id) => Some(M::Send(id)),
            _ => None,
        }?;
        Some(NetworkOpEvent(id, n))
    }
}

impl ProjectionUp<model::NetworkOp> for NetworkOpProjection {
    fn gen_state(&self, generator: &mut impl Generator, state: model::NetworkOp) -> Self::System {
        // TODO: set up peers
        state
            .iter()
            .map(|(id, phase)| {
                let mut node = system::NodeState::new();
                let state = match phase {
                    // OpOrigin is arbitrary
                    model::NodeOpPhase::Pending => {
                        Some({ system::OpState::Pending(system::OpOrigin::Fetched) })
                    }
                    model::NodeOpPhase::Validated => Some(system::OpState::Validated),
                    model::NodeOpPhase::Rejected => {
                        Some(system::OpState::Rejected("reason".into()))
                    }
                    model::NodeOpPhase::Integrated => Some(system::OpState::Integrated),
                    model::NodeOpPhase::None => None,
                };
                if let Some(state) = state {
                    node.vault.insert(
                        self.op.hash.clone(),
                        system::OpData {
                            op: self.op.clone(),
                            state,
                        },
                    );
                }
                (id.clone(), node)
            })
            .collect()
    }

    fn gen_event(
        &self,
        _generator: &mut impl Generator,
        model::NetworkOpEvent(id, event): model::NetworkOpEvent,
    ) -> Self::Event {
        use model::NodeOpEvent as M;
        use system::NodeEvent as S;
        match event {
            M::Store => (
                id,
                S::StoreOp(self.op.clone(), system::FetchDestination::Vault),
            ),
            M::Validate => (
                id,
                S::SetOpState(self.op.hash.clone(), system::OpState::Validated),
            ),
            M::Reject => (
                id,
                S::SetOpState(
                    self.op.hash.clone(),
                    system::OpState::Rejected("reason".into()),
                ),
            ),
            M::Integrate => (
                id,
                S::SetOpState(self.op.hash.clone(), system::OpState::Integrated),
            ),
            M::Send(id) => (id.clone(), S::SendOp(self.op.clone(), id)),
        }
    }
}

fn initial_state(ids: &[NodeId]) -> (HashMap<NodeId, system::NodeState>, Op) {
    let mut gen = proptest::test_runner::TestRunner::default();

    let mut system: HashMap<NodeId, system::NodeState> = ids
        .iter()
        .map(|id| (id.clone(), system::NodeState::new()))
        .collect();

    let (_, op) = system
        .iter_mut()
        .next()
        .map(|(id, n)| {
            for i in 0..5 {
                n.handle_event(system::NodeEvent::AuthorOp(0));
            }

            (id.clone(), n.vault.iter().next().unwrap().1.op.clone())
        })
        .unwrap();

    (system, op)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest::proptest! {
        #[test]
        fn test_invariants(event: model::NodeOpEvent) {
            let ids: Vec<_> = std::iter::repeat_with(Id::new).map(NodeId::from).take(3).collect();
            let mut gen = proptest::test_runner::TestRunner::default();
            let (system, op) = initial_state(&ids);
            let projection = NetworkOpProjection { op };
            let event = projection.gen_event(&mut gen, NetworkOpEvent(ids[0].clone(), event));
            projection.test_commutativity(
                system,
                event,
            );
        }
    }
}
