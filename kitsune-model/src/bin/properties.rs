use exhaustive::*;

use itertools::Itertools;
use kitsune_model::gossip::gossip_network::*;
use kitsune_model::gossip::gossip_node::*;
use polestar::machine::checked::Predicate as P;
use polestar::prelude::*;
use polestar::traversal::traverse_checked;
use polestar::traversal::TraversalConfig;
use tracing::Level;

fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::INFO)
        .init();

    type N = UpTo<3>;

    let machine = GossipMachine::<N>::new();

    // let initial = machine.initial();
    let initial = GossipState::new(
        [
            (N::new(0), NodeState::new([N::new(1), N::new(2)])),
            (N::new(1), NodeState::new([N::new(0)])),
            (N::new(2), NodeState::new([N::new(0)])),
        ]
        .into_iter()
        .collect(),
    );

    let ready = |n: N, p: N| {
        assert_ne!(n, p);
        P::atom(format!("{n}_ready_for_{p}"), move |s: &GossipState<N>| {
            s.nodes.get(&n).unwrap().schedule.get_key(&p).unwrap() == &PeerState::Ready
        })
    };

    let pairs = [(0, 1), (0, 2), (1, 0), (2, 0)]
        .into_iter()
        .map(|(n, p)| (N::new(n), N::new(p)));

    let liveness =
        pairs.filter_map(|(n, p)| (n != p).then(|| P::always(P::eventually(ready(n, p)))));

    let mut predicates = vec![];
    predicates.extend(liveness);

    let display_predicates = predicates.iter().map(|p| format!("{p:?}")).join("\n");
    let checker = machine.checked().with_predicates(predicates);
    let initial = checker.initial(initial);
    if let Err(err) = traverse_checked(
        checker,
        initial,
        TraversalConfig {
            max_depth: None,
            trace_every: 10_000,
            ..Default::default()
        },
    ) {
        eprintln!("{:#?}", err.path);
        eprintln!("{}", err.error);
        panic!("properties failed");
    };

    println!("properties satisfied:\n\n{}\n", display_predicates);
}
