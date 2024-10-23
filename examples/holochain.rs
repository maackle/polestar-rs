enum HolochainEvent {
    Init,
}

enum HolochainState {
    Uninitialized,
    ConductorInitialized(ConductorState),
}

enum ConductorEvent {
    Admin(AdminEvent),
    App(AppEvent),
    Cell(CellEvent),
}

struct ConductorState {
    apps: HashMap<AppId, AppState>,
    cells: HashMap<CellId, CellState>,
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

enum AppEvent {
    CreateCloneCell(Box<CreateCloneCellPayload>),
    EnableCloneCell { id: CloneId },
    DisableCloneCell { id: CloneId },
    // CallZome(CallZome),
    // ProvideMemproofs(MemproofMap),
    // EnableApp,
}

enum CellEvent {
    // CallZome(CallZome),
}

type AppId = String;
type CellId = (DnaHash, AgentKey);
type CloneId = DnaHash;

pub struct InstallAppPayload {}

pub struct CreateCloneCellPayload {
    pub role_name: RoleName,
}
