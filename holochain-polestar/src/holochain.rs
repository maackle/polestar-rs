mod app;
mod app_store;
mod cell;
mod cell_store;
mod conductor;

use crate::*;
use std::default;

use app::*;
use app_store::*;
use cell::*;
use cell_store::*;
use conductor::*;

use holo_hash::{AgentPubKey as AgentKey, *};

use anyhow::anyhow;
use polestar::prelude::*;

#[derive(Debug, derive_more::From)]
pub enum HolochainEvent {
    Init,
    Conductor(ConductorEvent),
}

#[derive(Default, derive_more::From)]
pub enum HolochainState {
    #[default]
    Uninitialized,
    ConductorInitialized(ConductorState),
}

impl polestar::Fsm for HolochainState {
    type Action = HolochainEvent;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(mut self, event: Self::Action) -> FsmResult<Self> {
        self = match event {
            HolochainEvent::Init => HolochainState::ConductorInitialized(ConductorState::default()),
            HolochainEvent::Conductor(e) => match self {
                HolochainState::ConductorInitialized(conductor) => conductor.transition_(e)?.into(),
                HolochainState::Uninitialized => {
                    anyhow::bail!("Conductor not initialized");
                }
            },
        };
        Ok((self, ()))
    }
}

type AppId = String;
type CloneId = DnaHash;
type RoleName = String;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
// #[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct CellId(pub DnaHash, pub AgentPubKey);

#[cfg(test)]
mod tests {
    use core::default::Default;

    use polestar::prelude::{Generator, Projection};
    use polestar::Fsm;
    use prop::test_runner::TestRunner;
    use proptest::prelude::*;
    use proptest::strategy::ValueTree;

    use super::*;

    struct ConductorStateDiagram;

    enum ConductorEventDiagram {
        InstallApp(InstallAppPayload),
    }

    #[test]
    fn test_init() {
        let mut g = TestRunner::default();
        let agent_key = g.generate().unwrap();
        let manifest = g.generate().unwrap();
        let mut h = HolochainState::default();

        let h = h.transition_(HolochainEvent::Init).unwrap();

        let h = h
            .transition_(
                ConductorEvent::Admin(AdminEvent::InstallApp(InstallAppPayload {
                    app_id: "test".to_string(),
                    agent_key,
                    manifest,
                }))
                .into(),
            )
            .unwrap();
    }
}
