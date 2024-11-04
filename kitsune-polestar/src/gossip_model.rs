use std::collections::HashMap;

use kitsune_p2p::{
    dependencies::kitsune_p2p_types::{KitsuneError, KitsuneResult},
    gossip::sharded_gossip::{
        store::AgentInfoSession, Initiate, ShardedGossipLocal, ShardedGossipWire,
    },
    NodeCert,
};
use polestar::prelude::*;
use proptest_derive::Arbitrary;

use crate::{
    block_on,
    round_model::{map_result, RoundEvent, RoundFsm},
};

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary, derive_more::From)]
pub struct GossipState {
    rounds: HashMap<NodeCert, RoundFsm>,
    initiate_tgt: Option<NodeCert>,
}

impl Fsm for GossipState {
    type Event = GossipEvent;
    type Fx = ();

    fn transition(&mut self, (node, event): Self::Event) {
        if let Some(round) = self.rounds.get_mut(&node) {
            round.transition(event);
        }
    }
}

pub type GossipEvent = (NodeCert, RoundEvent);

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

    fn map_event(&self, (node, msg): Self::Event) -> Option<GossipEvent> {
        crate::round_model::map_event(msg).map(|e| (node, e))
    }

    fn map_state(&self) -> Option<GossipState> {
        todo!()
        // Some(map_result(self.map(|s| {
        //     map_result(s.inner.share_mut(|s, _| {
        //         Ok(s.round_map
        //             .iter()
        //             .map(|(k, v)| (k, map_state(v.clone())))
        //             .collect()
        //             .into())
        //     }))
        // })))
    }

    fn gen_event(&self, generator: &mut impl Generator, event: GossipEvent) -> Self::Event {
        todo!()
    }

    fn gen_state(&self, generator: &mut impl Generator, state: GossipState) -> Self {
        todo!()
    }
}
