use std::sync::Arc;

use kitsune_p2p::{
    dependencies::kitsune_p2p_types::{GossipType, KitsuneResult},
    gossip::sharded_gossip::{store::AgentInfoSession, RoundState, ShardedGossipWire},
    NodeCert,
};
use polestar::{fsm::Contextual, prelude::*};
use proptest_derive::Arbitrary;

use crate::block_on;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub enum RoundPhase {
    Begin,
    AgentDiffReceived,
    AgentsReceived,
    OpDiffReceived,
    Finished,

    Error,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub enum RoundEvent {
    Initiate,
    Accept,
    AgentDiff,
    Agents,
    OpDiff,
    Ops,
    Close,
}

pub type RoundContext = GossipType;

impl Fsm for RoundPhase {
    type Event = (RoundEvent, Arc<RoundContext>);
    type Fx = ();

    fn transition(&mut self, (event, ctx): Self::Event) {
        use GossipType as T;
        use RoundEvent as E;
        use RoundPhase as P;
        polestar::util::update_replace(self, |s| {
            let next = match (*ctx, s, event) {
                (T::Recent, P::Begin, E::AgentDiff) => P::AgentDiffReceived,
                (T::Historical, P::Begin, E::OpDiff) => P::OpDiffReceived,
                (T::Recent, P::AgentDiffReceived, E::Agents) => P::AgentsReceived,
                (T::Recent, P::AgentsReceived, E::OpDiff) => P::OpDiffReceived,
                (_, P::OpDiffReceived, E::Ops) => P::Finished,

                // This might not be right
                (_, _, E::Close) => P::Finished,

                _ => P::Error,
            };
            (next, ())
        });
    }
}

pub type RoundFsm = Contextual<RoundPhase, RoundContext>;

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

pub fn map_state(state: RoundState) -> Option<RoundPhase> {
    todo!()
}

pub fn map_result(f: impl FnOnce() -> KitsuneResult<RoundPhase>) -> RoundPhase {
    match f() {
        Ok(s) => s,
        Err(e) => RoundPhase::Error,
    }
}

#[test]
fn diagram_round_state() {
    use polestar::diagram::*;

    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap();

    print_dot_state_diagram(
        RoundPhase::Begin.context(GossipType::Recent),
        vec![
            RoundPhase::Error.context(GossipType::Recent),
            RoundPhase::Finished.context(GossipType::Recent),
        ],
        1000,
    );

    print_dot_state_diagram(
        RoundPhase::Begin.context(GossipType::Historical),
        vec![
            RoundPhase::Error.context(GossipType::Historical),
            RoundPhase::Finished.context(GossipType::Historical),
        ],
        1000,
    );
}
