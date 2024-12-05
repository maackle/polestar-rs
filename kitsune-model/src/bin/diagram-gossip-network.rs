use itertools::Itertools;
use kitsune_model::gossip::gossip_network::*;
use kitsune_model::gossip::gossip_node::*;
use polestar::{
    diagram::exhaustive::*,
    id::{IdUnit, UpTo},
    machine::checked::Predicate as P,
    traversal::traverse_checked,
};

fn main() {
    // With 3 nodes, scheduled:                nodes=13824, edges=107136, finished in 52.91s
    // with 3 nodes, unscheduled with no tick: nodes=4096,  edges=18432,  finished in 68.64s
    type N = UpTo<2>;
    const TIMED: bool = true;

    let machine = GossipMachine::<N>::new();
    let initial = machine.initial();
    // let initial = GossipState::new(
    //     [
    //         (N::new(0), NodeState::new([N::new(1), N::new(2)])),
    //         (N::new(1), NodeState::new([N::new(0)])),
    //         (N::new(2), NodeState::new([N::new(0)])),
    //     ]
    //     .into_iter()
    //     .collect(),
    // );

    write_dot_state_diagram_mapped(
        "gossip-network.dot",
        machine,
        initial,
        &DiagramConfig {
            max_depth: None,
            ignore_loopbacks: true,
            ..Default::default()
        },
        |state| {
            Some({
                let lines = state
                    .nodes
                    .into_iter()
                    .map(|(n, s)| {
                        let s = NodeStateSimple::new(TIMED, s);
                        format!("{s}")
                            .split('\n')
                            .filter_map(|l| (!l.is_empty()).then_some(format!("{n}.{l}")))
                            .join("\n")
                    })
                    .collect_vec()
                    .join("\n");
                format!("{lines}\n")
            })
        },
        |GossipAction(node, action)| {
            Some(format!("{node}: {}", NodeAction::<N, IdUnit>::from(action)))
        },
        // |GossipAction(node, action)| {
        //     (!matches!(action, NodeAction::Tick)).then_some(format!("{node}: {action}"))
        // },
    );
}
