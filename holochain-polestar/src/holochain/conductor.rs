use super::*;
use crate::*;

#[derive(Debug, derive_more::From)]
pub enum ConductorEvent {
    Admin(AdminEvent),
    App(AppId, AppEvent),
    // Cell(CellId, CellEvent),
}

#[derive(Default)]
pub struct ConductorState {
    apps: polestar::actor::ActorRw<AppStore>,
    cells: polestar::actor::ActorRw<CellStore>,
}

impl polestar::Fsm for ConductorState {
    type Event = ConductorEvent;
    type Fx = ();

    fn transition(&mut self, event: Self::Event) -> Self::Fx {
        match event {
            ConductorEvent::Admin(e) => {
                match e {
                    AdminEvent::InstallApp(payload) => {
                        // TODO: how to glue together this substate transition with the global one that caused it?
                        self.apps.transition(AppStoreEvent::InstallApp(
                            payload.app_id,
                            AppContext::new(payload.agent_key, payload.manifest),
                        ));
                    }
                    _ => {
                        todo!()
                    }
                }
                // self.apps.transition(e);
            }
            ConductorEvent::App(id, e) => {
                self.apps.transition(AppStoreEvent::AppEvent(id, e));
            } // ConductorEvent::Cell(id, e) => {
              //     self.cells.transition(CellStoreEvent::CellEvent(id, e));
              // }
        }
    }
}

#[derive(Debug)]
pub enum AdminEvent {
    InstallApp(InstallAppPayload),
    UninstallApp(AppId),
    EnableApp { app_id: AppId },
    DisableApp { app_id: AppId },
    // DeleteCloneCell { app_id: AppId, clone_id: CloneId },
    // GenerateAgentKey,
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
