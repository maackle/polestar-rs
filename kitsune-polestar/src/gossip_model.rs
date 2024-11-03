use kitsune_p2p::{
    dependencies::kitsune_p2p_types::{KitsuneError, KitsuneResult},
    gossip::sharded_gossip::{store::AgentInfoSession, ShardedGossipLocal, ShardedGossipWire},
    NodeCert,
};
use polestar::prelude::*;
use proptest_derive::Arbitrary;

use crate::block_on;

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
pub enum GossipState {
    Initiated,
    Accepted,
    AgentDiffReceived,
    AgentsReceived,
    OpDiffReceived,
    OpsReceived,
    Error,
}

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
pub enum GossipEvent {
    Initiate,
    Accept,
    AgentDiff,
    Agents,
    OpDiff,
    Ops,
    Close,
}

impl Fsm for GossipState {
    type Event = GossipEvent;
    type Fx = ();

    fn transition(&mut self, event: Self::Event) {}
}

impl Projection<GossipState> for KitsuneResult<ShardedGossipLocal> {
    type Event = (NodeCert, ShardedGossipWire);

    fn apply(&mut self, (node, msg): Self::Event) {
        if let Ok(g) = self {
            let mut session = AgentInfoSession::default();
            let r = block_on(g.process_incoming(node, msg, &mut session));
            match r {
                Ok(_) => (),
                Err(e) => {
                    *self = Err(e);
                }
            }
        }
    }

    fn map_event(&self, (node, msg): Self::Event) -> GossipEvent {
        match msg {
            ShardedGossipWire::Initiate(initiate) => GossipEvent::Initiate,
            ShardedGossipWire::Accept(accept) => GossipEvent::Accept,
            ShardedGossipWire::Agents(agents) => GossipEvent::AgentDiff,
            ShardedGossipWire::MissingAgents(missing_agents) => GossipEvent::Agents,
            ShardedGossipWire::OpBloom(op_bloom) => GossipEvent::OpDiff,
            ShardedGossipWire::OpRegions(op_regions) => GossipEvent::OpDiff,
            ShardedGossipWire::MissingOpHashes(missing_op_hashes) => GossipEvent::Ops,
            ShardedGossipWire::OpBatchReceived(op_batch_received) => todo!(),

            ShardedGossipWire::Error(_)
            | ShardedGossipWire::Busy(_)
            | ShardedGossipWire::NoAgents(_)
            | ShardedGossipWire::AlreadyInProgress(_) => GossipEvent::Close,
        }
    }

    fn map_state(&self) -> GossipState {
        todo!()
    }

    fn gen_event(&self, generator: &mut impl Generator, event: GossipEvent) -> Self::Event {
        todo!()
    }

    fn gen_state(&self, generator: &mut impl Generator, state: GossipState) -> Self {
        todo!()
    }
}
