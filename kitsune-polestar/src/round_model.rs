use kitsune_p2p::{
    dependencies::kitsune_p2p_types::KitsuneResult,
    gossip::sharded_gossip::{store::AgentInfoSession, RoundState, ShardedGossipWire},
    NodeCert,
};
use polestar::prelude::*;
use proptest_derive::Arbitrary;

use crate::block_on;

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
pub enum RoundFsm {
    Initiated,
    Accepted,
    AgentDiffReceived,
    AgentsReceived,
    OpDiffReceived,
    OpsReceived,
    Error,
}

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
pub enum RoundEvent {
    Initiate,
    Accept,
    AgentDiff,
    Agents,
    OpDiff,
    Ops,
    Close,
}

impl Fsm for RoundFsm {
    type Event = RoundEvent;
    type Fx = ();

    fn transition(&mut self, event: Self::Event) {
        todo!()
    }
}

pub fn map_event(msg: ShardedGossipWire) -> Option<RoundEvent> {
    match msg {
        ShardedGossipWire::Initiate(initiate) => Some(RoundEvent::Initiate),
        ShardedGossipWire::Accept(accept) => Some(RoundEvent::Accept),
        ShardedGossipWire::Agents(agents) => Some(RoundEvent::AgentDiff),
        ShardedGossipWire::MissingAgents(missing_agents) => Some(RoundEvent::Agents),
        ShardedGossipWire::OpBloom(op_bloom) => Some(RoundEvent::OpDiff),
        ShardedGossipWire::OpRegions(op_regions) => Some(RoundEvent::OpDiff),
        ShardedGossipWire::MissingOpHashes(missing_op_hashes) => Some(RoundEvent::Ops),
        ShardedGossipWire::OpBatchReceived(op_batch_received) => None,

        ShardedGossipWire::Error(_)
        | ShardedGossipWire::Busy(_)
        | ShardedGossipWire::NoAgents(_)
        | ShardedGossipWire::AlreadyInProgress(_) => Some(RoundEvent::Close),
    }
}

pub fn map_state(state: RoundState) -> Option<RoundFsm> {
    todo!()
}

pub fn map_result(f: impl FnOnce() -> KitsuneResult<RoundFsm>) -> RoundFsm {
    match f() {
        Ok(s) => s,
        Err(e) => RoundFsm::Error,
    }
}
