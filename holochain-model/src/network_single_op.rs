

use std::collections::BTreeSet;
use std::collections::BTreeMap;

use anyhow::bail;
use exhaustive::Exhaustive;
use itertools::Itertools;
use polestar::{id::IdT, util::transition_btreemap};
use polestar::prelude::*;

use crate::single_op::OpMachine;

use super::single_op::{OpPhase, OpEvent, ValidationType as VT};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NetworkMachine<NodeId: IdT, OpId: IdT> {
    sub: OpMachine<NodeId, OpId>,
}

impl<NodeId: IdT, OpId: IdT> NetworkMachine<NodeId, OpId> {
    /// Create a new OpMachine with the given dependencies
    pub fn new(deps: BTreeSet<OpId>) -> Self {
        let id = todo!();
        Self {  
            sub: OpMachine::new(id, deps),
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq, Hash, derive_more::Deref)]
pub struct NetworkState<NodeId: IdT, OpId: IdT>{
    nodes: BTreeMap<NodeId, OpPhase<OpId>>,
}

impl<NodeId: IdT, OpId: IdT> NetworkState<NodeId, OpId>{
    pub fn new(nodes: BTreeMap<NodeId, OpPhase<OpId>>) -> Self {
        Self {
            nodes: BTreeMap::from(nodes),
        }
    }

    pub fn new_empty(ids: &[NodeId]) -> Self {
        let nodes: BTreeMap<NodeId, _> = ids
            .iter()
            .map(|id| (id.clone(), Default::default()))
            .collect();
        Self {
            nodes: BTreeMap::from(nodes),
        }
    }
}

impl<NodeId: IdT, OpId: IdT> Machine for NetworkMachine<NodeId, OpId>{
    type State = NetworkState<NodeId, OpId>;
    type Action = NetworkOpEvent<NodeId, OpId>;
    type Fx = ();
    type Error = String;

    fn transition(
        &self,
        mut state: Self::State,
        NetworkOpEvent(node_id, event): Self::Action,
    ) -> MachineResult<Self> {

        let honest = true;

        if let OpEvent::Send(id) = &event {
            if !state
                .nodes
                .values()
                .any(|n| matches!(n, OpPhase::Integrated))
            {
                return Err(
                    "a Send can't happen until at least one node has Integrated the op".to_string(),
                );
            }

            if node_id == *id {
                return Err("cannot send op to self".to_string());
            }

            let node = state.nodes.get_mut(id).unwrap();
            match node {
                OpPhase::None => *node = OpPhase::Pending,
                _ => return Err("don't send op twice".to_string()),
            }
        }

        if let OpEvent::Author = event {
            if state.nodes.values().any(|n| !matches!(n, OpPhase::None)) {
                return Err("this model only handles one Author event".to_string());
            }
        }

        if honest {

            if matches!(event, OpEvent::Reject) && state
                    .nodes
                .values()
                .any(|n| matches!(n, OpPhase::Validated(VT::App) | OpPhase::Integrated))
            {
                return Err("No honest node will reject if other nodes have validated".to_string());
            }
            
            if matches!(event, OpEvent::Validate(_)) && state
                    .nodes
                .values()
                .any(|n| matches!(n, OpPhase::Rejected))
            {
                return Err("No honest node will validate if other nodes have rejected".to_string());
            }
        }

        transition_btreemap(&self.sub, node_id, &mut state.nodes, event)
            .ok_or_else(|| format!("no node {:?}", node_id))?
            .map_err(|e| format!("{:?}", e))?;
        Ok((state, ()))
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        let vs = || state.nodes.values();

        // all integrated
        vs().all(|n| matches!(n, OpPhase::Integrated))
        || (
            // all non-None are rejected
            vs().all(|n| matches!(n, OpPhase::None | OpPhase::Rejected) 
            // but not all None
            && vs().any(|n| !matches!(n, OpPhase::None)))
        )
    }
}

impl<NodeId: IdT, OpId: IdT> std::fmt::Debug for NetworkState<NodeId, OpId> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (id, n) in self.nodes.iter() {
            writeln!(f, "{}: {:?}", id, n)?;
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Exhaustive)]
pub struct NetworkOpEvent<NodeId: IdT, OpId: IdT>(pub NodeId, pub OpEvent<NodeId, OpId>);

impl<NodeId: IdT, OpId: IdT> std::fmt::Debug for NetworkOpEvent<NodeId, OpId> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.1 {
            OpEvent::Send(id) => write!(f, "Send({}↦{})", self.0, id),
            _ => write!(f, "{:?}({})", self.1, self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use polestar::{diagram::exhaustive::write_dot_state_diagram, id::Id};
    use super::*;

    #[test]
    #[ignore = "diagram"]
    fn test_diagram() {
        use polestar::diagram::exhaustive::DiagramConfig;

        // tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

        type NodeId = Id<3>;
        type OpId = Id<3>;

        let ids = Id::<3>::iter_exhaustive(None).collect_vec();
        let machine = NetworkMachine::<NodeId, OpId>::new(BTreeSet::new());
        let (initial, ()) = machine.transition(NetworkState::new_empty(&ids), NetworkOpEvent(ids[0].clone(), OpEvent::Author))
            .unwrap();

        // TODO allow for strategy params
        write_dot_state_diagram(
            "network-single-op.dot",
            machine,
            initial,
            &DiagramConfig {
                ignore_loopbacks: true,
                ..Default::default()
            },
        );
    }
}
