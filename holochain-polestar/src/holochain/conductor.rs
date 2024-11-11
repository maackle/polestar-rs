use super::*;
use crate::*;

use polestar::prelude::*;

#[derive(Debug, derive_more::From)]
pub enum ConductorEvent {
    Admin(AdminEvent),
    App(AppId, AppEvent),
    // Cell(CellId, CellEvent),
}

#[derive(Default, Clone)]
pub struct ConductorState {
    apps: polestar::actor::ActorFsm<AppStore>,
    cells: polestar::actor::ActorFsm<CellStore>,
}

impl Fsm for ConductorState {
    type Action = ConductorEvent;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(mut self, event: Self::Action) -> FsmResult<Self> {
        let (_, _fx) = match event {
            ConductorEvent::Admin(e) => {
                match e {
                    AdminEvent::InstallApp(payload) => {
                        // TODO: how to glue together this substate transition with the global one that caused it?
                        self.apps.clone().transition(AppStoreEvent::InstallApp(
                            payload.app_id,
                            AppContext::new(payload.agent_key, payload.manifest),
                        ))?
                    }
                    _ => {
                        todo!()
                    }
                }
                // self.apps.transition(e);
            }
            ConductorEvent::App(id, e) => self
                .apps
                .clone()
                .transition(AppStoreEvent::AppEvent(id, e))?,
            // ConductorEvent::Cell(id, e) => {
            //     self.cells.transition(CellStoreEvent::CellEvent(id, e));
            // }
        };
        todo!("handle fx");
        Ok((self, ()))
    }
}

#[derive(Debug)]
pub enum AdminEvent {
    InstallApp(InstallAppPayload),
    UninstallApp(AppId),
    EnableApp { app_id: AppId },
    DisableApp { app_id: AppId },
    GenerateAgentKey,
    // DeleteCloneCell { app_id: AppId, clone_id: CloneId },
    // AttachAppInterface {
    //     port: Option<u16>,
    //     allowed_origins: AllowedOrigins,
    //     app_id: Option<AppId>,
    // },
    // AddAgentInfo {
    //     /// list of signed agent info to add to peer store
    //     agent_infos: Vec<AgentInfoSigned>,
    // },
    // GrantZomeCallCapability(Box<GrantZomeCallCapabilityPayload>),
    // IssueAppAuthenticationToken(IssueAppAuthenticationTokenPayload),
    // RevokeAppAuthenticationToken(AppAuthenticationToken),
}
