use itertools::Itertools;
use polestar::Machine;

use crate::{
    op_family::{OpFamilyPhase, OpFamilyState},
    op_network::{OpNetworkMachine, OpNetworkMachineAction, OpNetworkState},
    op_single::{OpPhase, Outcome},
};

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

    let machine = OpNetworkMachine::<N, O, T>::new();
    dbg!(&machine);

    let initial = machine.initial();

    match machine.apply_each_action(initial, actions, |a, s| {
        eprintln!("action: {:?}", a);
    }) {
        Err((e, s, a)) => {
            eprintln!("state: {:#?}, action: {:#?}", s, a);
            panic!("{e:?}");
        }
        Ok((state, _)) => {
            let mut expected = OpNetworkState::default();
            for (n, ops) in state.nodes.iter() {
                expected.nodes.insert(n.clone(), OpFamilyState::default());
                for (op, _) in ops.iter() {
                    expected.nodes.get_mut(n).unwrap().insert(
                        op.clone(),
                        OpFamilyPhase::Op(OpPhase::Integrated(Outcome::Accepted)),
                    );
                }
            }

            pretty_assertions::assert_eq!(state, expected, "all ops must be integrated");
            dbg!(&state);
        }
    }
}
