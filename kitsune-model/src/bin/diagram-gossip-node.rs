use std::collections::BTreeMap;
use std::fmt::Display;

use itertools::Itertools;
use kitsune_model::gossip::gossip_network::*;
use kitsune_model::gossip::gossip_node::*;
use polestar::{
    diagram::exhaustive::*, machine::checked::Predicate as P, prelude::*,
    traversal::traverse_checked,
};
use tracing::Level;

fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // type N = IdUnit;
    type N = UpTo<1>;

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
        // Some,
        |s| Some(NodeStateSimple::new(true, s)),
        // Some,
        |a| Some(NodeAction::<N, IdUnit>::from(a)),
        // |a| (!matches!(a, NodeAction::Tick)).then_some(a),
    );
}
