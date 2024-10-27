mod app;
mod app_store;
mod cell;
mod cell_store;
mod conductor;

use std::default;

use app::*;
use app_store::*;
use cell::*;
use cell_store::*;
use conductor::*;

use holo_hash::{AgentPubKey as AgentKey, *};

use anyhow::anyhow;

#[derive(Debug, derive_more::From)]
pub enum HolochainEvent {
    Init,
    Conductor(ConductorEvent),
}

#[derive(Default)]
pub enum HolochainState {
    #[default]
    Uninitialized,
    ConductorInitialized(ConductorState),
}

impl polestar::Fsm for HolochainState {
    type Event = HolochainEvent;
    type Fx = anyhow::Result<()>;

    fn transition(&mut self, event: Self::Event) -> Self::Fx {
        match event {
            HolochainEvent::Init => {
                *self = HolochainState::ConductorInitialized(ConductorState::default());
            }
            HolochainEvent::Conductor(e) => match self {
                HolochainState::ConductorInitialized(ref mut conductor) => {
                    conductor.transition(e);
                }
                HolochainState::Uninitialized => {
                    anyhow::bail!("Conductor not initialized");
                }
            },
        }
        Ok(())
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

    use polestar::prelude::Generate;
    use polestar::Fsm;
    use prop::test_runner::TestRunner;
    use proptest::prelude::*;
    use proptest::strategy::ValueTree;

    use super::*;

    #[test]
    fn test_init() {
        let mut g = TestRunner::default();
        let agent_key = g.generate().unwrap();
        let manifest = g.generate().unwrap();
        let mut h = HolochainState::default();

        h.transition(HolochainEvent::Init).unwrap();

        h.transition(
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
