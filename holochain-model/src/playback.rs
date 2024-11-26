use itertools::Itertools;
use polestar::Machine;

use crate::op_network::{OpNetworkMachine, OpNetworkMachineAction};

#[test]
fn test_playback() {
    type N = u32;
    type O = u32;
    let path = "/home/michael/Holo/chain/crates/holochain/op_events.json";
    let text = std::fs::read_to_string(path).unwrap();
    let text = text.lines().join(",");
    let json = format!("[{}]", text);

    let actions: Vec<OpNetworkMachineAction<N, O>> = serde_json::from_str(&json).unwrap();

    dbg!(&actions);

    let machine = OpNetworkMachine::<N, O>::new();

    let initial = machine.initial();

    let state = machine.apply_actions_(initial, actions).unwrap();
    dbg!(&state);
}
