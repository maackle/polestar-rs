use std::collections::BTreeMap;
use std::fmt::Display;

use itertools::Itertools;
use kitsune_model::gossip::gossip_network::*;
use kitsune_model::gossip::gossip_node::*;
use polestar::{
    diagram::exhaustive::*, machine::checked::Predicate as P, prelude::*,
    traversal::traverse_checked,
};

fn main() {
    // type N = IdUnit;
    type N = UpTo<2>;

    let machine = NodeMachine::<N>::new();
    let state = machine.initial();

    write_dot_state_diagram_mapped(
        "gossip-node.dot",
        machine,
        state,
        &DiagramConfig {
            max_depth: None,
            ..Default::default()
        },
        |s| Some(NodeStateUnscheduled::from(s)),
        Some,
        // |a| (!matches!(a, NodeAction::Tick)).then_some(a),
    );
}
