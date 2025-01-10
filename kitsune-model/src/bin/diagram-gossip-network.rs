use itertools::Itertools;
use kitsune_model::gossip::gossip_network::*;
use kitsune_model::gossip::gossip_node::*;
use polestar::diagram::write_dot;
use polestar::id::{IdUnit, UpTo};
use polestar::traversal::traverse;
use polestar::traversal::TraversalConfig;

fn main() {
    // With 3 nodes, scheduled:                 nodes=13824, edges=107136, finished in 52.91s
    // with 3 nodes, unscheduled with no tick:  nodes=4096,  edges=18432,  finished in 68.64s
    type N = UpTo<2>;
    const TIMED: bool = true;

    let machine = GossipMachine::<N>::new();
    let initial = machine.initial();
    let config = TraversalConfig::default();

    let (_, graph, _) = traverse(machine.into(), initial, config, Some).unwrap();

    let graph = graph.unwrap().map(
        |_, state| {
            let lines = state
                .nodes
                .iter()
                .map(|(n, s)| {
                    let s = NodeStateSimple::new(TIMED, s);
                    format!("{s}")
                        .split('\n')
                        .filter_map(|l| (!l.is_empty()).then_some(format!("{n}↤{l}")))
                        .join("\n")
                })
                .collect_vec()
                .join("\n");
            format!("{lines}\n")
        },
        |_, GossipAction(node, action)| {
            format!("{node}: {}", NodeAction::<N, IdUnit>::from(*action))
        },
    );

    write_dot("gossip-network.dot", &graph, &[]);
}
