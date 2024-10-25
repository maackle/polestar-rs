mod app;
mod cell;
mod conductor;

use app::*;
use cell::*;
use conductor::*;

use holo_hash::{AgentPubKey as AgentKey, *};

enum HolochainEvent {
    Init,
}

enum HolochainState {
    Uninitialized,
    ConductorInitialized(ConductorState),
}

type AppId = String;
type CellId = (DnaHash, AgentKey);
type CloneId = DnaHash;
type RoleName = String;
