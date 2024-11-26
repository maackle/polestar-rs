use itertools::Itertools;
use polestar::Machine;

use crate::op_network::{OpNetworkMachine, OpNetworkMachineAction};

#[test]
fn test_playback() {
    type N = u32;
    type O = u32;
    type T = u8;
    // let path = "/home/michael/Holo/chain/crates/holochain/op_events.json";
    let path = "/tmp/op-events.json";
    // let path = "/home/michael/proj/polestar-rs/op-events.json";
    let text = std::fs::read_to_string(path).unwrap();
    let text = text.lines().join(",");
    let json = format!("[{}]", text);

    let actions: Vec<OpNetworkMachineAction<N, O, T>> = serde_json::from_str(&json).unwrap();

    for a in actions.iter() {
        println!("{a:?}");
    }

    let machine = OpNetworkMachine::<N, O, T>::new();

    let initial = machine.initial();

    match machine.apply_actions_(initial, actions) {
        Err((e, s, a)) => {
            panic!("{} state: {:#?}, action: {:#?}", e, s, a);
        }
        Ok(state) => {
            dbg!(&state);
        }
    }
}
