use std::collections::HashMap;

use anyhow::anyhow;
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
    round_model::{RoundEvent, RoundFsm},
};

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary, derive_more::From)]
pub struct GossipModel {
    rounds: FsmHashMap<NodeCert, RoundFsm>,
    initiate_tgt: Option<Tgt>,
}

impl Machine for GossipModel {
    type Action = GossipEvent;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(mut self, (node, event): Self::Action) -> MachineResult<Self> {
        self.rounds
            .transition_mut(node.clone(), event)
            .ok_or(anyhow!("no round for {node:?}"))?
            .map_err(|o| o.unwrap_or(anyhow!("terminal")))?;
        Ok((self, ()))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary, derive_more::From)]
pub struct Tgt {
    pub cert: NodeCert,
    pub tie_break: u32,
}

pub type GossipEvent = (NodeCert, RoundEvent);

pub struct GossipProjection;

impl Projection for GossipProjection {
    type System = ShardedGossipLocal;
    type Model = GossipModel;
    type Event = (NodeCert, ShardedGossipWire);

    fn apply(&self, system: &mut Self::System, (node, msg): Self::Event) {
        let mut session = AgentInfoSession::default();
        let r = block_on(system.process_incoming(node, msg, &mut session));
    }

    fn map_event(&self, (node, msg): Self::Event) -> Option<GossipEvent> {
        crate::round_model::map_event(msg).map(|e| (node, e))
    }

    fn map_state(&self, system: &Self::System) -> Option<GossipModel> {
        let state = system
            .inner
            .share_mut(|s, _| {
                let rounds = s
                    .round_map
                    .map
                    .iter()
                    .map(|(k, mut v)| {
                        (
                            k.clone(),
                            crate::round_model::map_state(v.clone())
                                .unwrap()
                                .context(system.gossip_type),
                        )
                    })
                    .collect::<HashMap<NodeCert, RoundFsm>>()
                    .into();

                let initiate_tgt = s.initiate_tgt.as_ref().map(|t| Tgt {
                    cert: t.cert.clone(),
                    tie_break: t.tie_break,
                });
                Ok(GossipModel {
                    rounds,
                    initiate_tgt,
                })
            })
            .unwrap();
        Some(state)
    }

    fn gen_event(&self, generator: &mut impl Generator, event: GossipEvent) -> Self::Event {
        unimplemented!("generation not implemented")
    }

    fn gen_state(&self, generator: &mut impl Generator, state: GossipModel) -> Self::System {
        unimplemented!("generation not implemented")
    }
}
