use std::collections::HashSet;

use itertools::Itertools;
use polestar::{id::Id, Machine};

use crate::{
    op_family::{OpFamilyPhase, OpFamilyState},
    op_network::{OpNetworkMachine, OpNetworkMachineAction, OpNetworkState},
    op_single::{OpPhase, Outcome},
};

#[cfg(test)]

mod tests {
    use super::*;

    fn assert_received_integrated<N, O, T>(state: &OpNetworkState<N, O, T>)
    where
        N: Id,
        O: Id,
        T: Id,
    {
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

        pretty_assertions::assert_eq!(*state, expected, "all received ops must be integrated");
    }

    fn assert_all_integrated<N, O, T>(state: &OpNetworkState<N, O, T>)
    where
        N: Id,
        O: Id,
        T: Id,
    {
        let mut nodes = HashSet::new();
        let mut ops = HashSet::new();

        for (n, op_states) in state.nodes.iter() {
            nodes.insert(n.clone());
            ops.extend(op_states.keys().cloned());
        }

        let mut expected = OpNetworkState::default();
        for n in nodes {
            expected.nodes.insert(n.clone(), OpFamilyState::default());
            for op in ops.iter() {
                expected.nodes.get_mut(&n).unwrap().insert(
                    *op,
                    OpFamilyPhase::Op(OpPhase::Integrated(Outcome::Accepted)),
                );
            }
        }

        pretty_assertions::assert_eq!(*state, expected, "all ops must be integrated");
    }

    #[test]
    fn test_playback() {
        type N = u32;
        type O = u32;
        type T = u8;
        let path = "/tmp/op-events.json";
        // let path = "/home/michael/Holo/chain/crates/holochain/op_events.json";
        // let path = "/home/michael/Downloads/op-events.json";
        // let path = "/home/michael/proj/polestar-rs/op-events.json";
        let text = std::fs::read_to_string(path).unwrap();
        assert!(!text.is_empty(), "events file is empty");
        let text = text
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .join(",");
        let json = format!("[{}]", text);

        let actions: Vec<OpNetworkMachineAction<N, O, T>> = serde_json::from_str(&json).unwrap();

        let machine = OpNetworkMachine::<N, O, T>::new();
        dbg!(&machine);

        let initial = machine.initial();

        match machine.apply_each_action(initial, actions, |a, _s| {
            // eprintln!("action: {:?}", a);
        }) {
            Err((e, s, a)) => {
                eprintln!("state: {:#?}, action: {:#?}", s, a);
                panic!("{e:?}");
            }
            Ok((state, _)) => {
                // assert_received_integrated(&state);
                assert_all_integrated(&state);
                dbg!(&state);
            }
        }
    }
}
