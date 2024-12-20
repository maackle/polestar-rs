use anyhow::anyhow;
use proptest_derive::Arbitrary;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::*;
use crate::*;

impl polestar::Machine for AppState {
    type Action = (AppEvent, Arc<AppContext>);
    type Fx = Option<AppFx>;
    type Error = anyhow::Error;

    fn transition(
        mut self,
        (event, context): Self::Action,
    ) -> Result<(Self, Self::Fx), Self::Error> {
        match event {
            AppEvent::Enable => {
                self.status = AppStatus::Running;
            }
            AppEvent::Disable => {
                self.status = AppStatus::Disabled;
            }
            AppEvent::CreateCloneCell { role_name } => {
                let dna_hash = context
                    .manifest
                    .roles
                    .get(&role_name)
                    .ok_or(anyhow!("Role not found: {role_name}"))?;
                // insert disabled
                self.clones.insert(context.cell_id(dna_hash), false);
            }
            AppEvent::EnableCloneCell { id } => {
                self.clones
                    .get_mut(&context.cell_id(&id))
                    .ok_or(anyhow!("clone not found, {id}"))?
                    .transition(true);
            }
            AppEvent::DisableCloneCell { id } => {
                self.clones
                    .get_mut(&context.cell_id(&id))
                    .ok_or(anyhow!("clone not found, {id}"))?
                    .transition(false);
            }
        }
        Ok((self, Some(AppFx::AdjustCells)))
    }
}

#[derive(Debug)]
pub enum AppEvent {
    Enable,
    Disable,
    CreateCloneCell { role_name: RoleName },
    EnableCloneCell { id: CloneId },
    DisableCloneCell { id: CloneId },
    // CallZome(CallZome),
    // ProvideMemproofs(MemproofMap),
    // EnableApp,
}

#[derive(Debug)]
pub enum AppFx {
    AdjustCells,
}

#[derive(Default)]
pub struct AppState {
    status: AppStatus,
    clones: HashMap<CellId, CloneState>,
}

impl AppState {
    pub fn required_cells(&self) -> HashSet<&CellId> {
        self.clones
            .iter()
            .filter_map(|(id, enabled)| enabled.then_some(id))
            .collect()
    }
}

/// Whether a clone cell is enabled or not.
pub type CloneState = bool;

#[derive(Default)]
pub enum AppStatus {
    #[default]
    Disabled,
    Paused,
    Running,
}

// impl Default for AppStatus {
//     fn default() -> Self {
//         AppStatus::Disabled
//     }
// }

pub struct AppContext {
    /// The agent key bound to this app.
    agent_key: AgentKey,
    /// The manifest of the app when it was installed.
    manifest: AppManifest,
    /// Cells bound to this app.
    /// These cells must be running for this app to be running in this conductor.
    cells: HashSet<CellId>,
    // /// Reference to the cell actor
    // cell_actor: CellActor,
}

impl AppContext {
    pub fn cell_id(&self, dna_hash: &DnaHash) -> CellId {
        CellId(dna_hash.clone(), self.agent_key.clone())
    }

    pub fn new(agent_key: AgentKey, manifest: AppManifest) -> Self {
        Self {
            agent_key,
            manifest,
            cells: HashSet::new(),
            // cell_actor: CellActor::new(),
        }
    }
}

#[derive(Debug, Arbitrary)]
pub struct AppManifest {
    roles: HashMap<RoleName, DnaHash>,
}

pub type AppFsm = polestar::fsm::Contextual<AppState, AppContext>;

#[derive(Debug)]
pub struct InstallAppPayload {
    pub app_id: AppId,
    pub agent_key: AgentKey,
    pub manifest: AppManifest,
}

#[derive(Debug)]
pub struct CreateCloneCellPayload {
    pub role_name: RoleName,
}
