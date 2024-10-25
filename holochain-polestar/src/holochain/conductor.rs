use super::*;
use crate::*;

enum ConductorEvent {
    Admin(AdminEvent),
    App(AppEvent),
    Cell(CellEvent),
}

pub(crate) struct ConductorState {
    apps: HashMap<AppId, AppFsm>,
    cells: HashMap<CellId, CellFsm>,
}

enum AdminEvent {
    InstallApp(InstallAppPayload),
    UninstallApp(AppId),
    EnableApp { app_id: AppId },
    DisableApp { app_id: AppId },
    DeleteCloneCell { app_id: AppId, clone_id: CloneId },
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
