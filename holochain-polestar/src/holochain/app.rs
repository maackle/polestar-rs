use std::collections::{HashMap, HashSet};

use super::*;
use crate::*;

pub enum AppEvent {
    Enable,
    Disable,
    CreateCloneCell(Box<CreateCloneCellPayload>),
    EnableCloneCell { id: CloneId },
    DisableCloneCell { id: CloneId },
    // CallZome(CallZome),
    // ProvideMemproofs(MemproofMap),
    // EnableApp,
}

pub struct AppState {
    status: AppStatus,
    clones: HashSet<CellId>,
}

pub enum AppStatus {
    Disabled,
    Paused,
    Running,
}

pub struct AppMeta {
    /// The manifest of the app when it was installed.
    manifest: AppManifest,
    /// Cells bound to this app.
    /// These cells must be running for this app to be running in this conductor.
    cells: HashSet<CellId>,
}

pub struct AppManifest {
    roles: HashMap<RoleName, DnaHash>,
}

pub type AppFsm = Fsm<AppState, AppEvent, AppMeta>;

pub struct InstallAppPayload {}

pub struct CreateCloneCellPayload {
    pub role_name: RoleName,
}
