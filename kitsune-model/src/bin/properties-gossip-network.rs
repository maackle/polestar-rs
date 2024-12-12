use exhaustive::*;

use itertools::Itertools;
use kitsune_model::gossip::gossip_network::*;
use kitsune_model::gossip::gossip_node::*;
use polestar::machine::checked::Predicate as P;
use polestar::prelude::*;
use polestar::traversal::TraversalConfig;
use tracing::Level;

fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    type N = UpTo<2>;

    let machine = GossipMachine::<N>::new();

    let initial = machine.initial();
    let pairs = N::iter_exhaustive(None)
        .combinations(2)
        .map(|c| <[N; 2]>::try_from(c).unwrap())
        .flat_map(|[a, b]| [[a, b], [b, a]])
        .collect_vec();

    // let initial = GossipState::new(
    //     [
    //         (N::new(0), NodeState::new([N::new(1), N::new(2)])),
    //         (N::new(1), NodeState::new([N::new(0)])),
    //         (N::new(2), NodeState::new([N::new(0)])),
    //     ]
    //     .into_iter()
    //     .collect(),
    // );
    // let pairs = [(0, 1), (0, 2), (1, 0), (2, 0)]
    //     .into_iter()
    //     .map(|(n, p)| (N::new(n), N::new(p)));

    fn peer_focus(s: &GossipState<N>, n: N, p: N) -> &PeerState {
        s.nodes.get(&n).unwrap().peers.get(&p).unwrap()
    }

    let ready = |n: N, p: N| {
        assert_ne!(n, p);
        P::atom(format!("ready({n},{p})"), move |s: &GossipState<N>| {
            peer_focus(s, n, p).phase == PeerPhase::Ready
        })
    };

    let premature = |n: N, p: N| {
        assert_ne!(n, p);
        P::atom2(format!("early({n},{p})"), move |a, b| {
            let a = peer_focus(a, n, p);
            let b = peer_focus(b, n, p);
            match (a.phase, b.phase) {
                (PeerPhase::Closed(GossipOutcome::Failure(_)), PeerPhase::Active) => true,
                (PeerPhase::Closed(GossipOutcome::Success(_)), PeerPhase::Active) => true,
                (
                    PeerPhase::Active,
                    PeerPhase::Closed(GossipOutcome::Failure(FailureReason::Timeout)),
                ) => a.timer > 0,
                _ => false,
            }
        })
    };

    let time_decreases = |n: N, p: N| {
        assert_ne!(n, p);
        P::atom2(format!("time_decreases({n},{p})"), move |a, b| {
            let a = peer_focus(a, n, p);
            let b = peer_focus(b, n, p);
            a.phase != b.phase || b.timer <= a.timer
        })
    };

    let safety = pairs
        .iter()
        .copied()
        .filter_map(|[n, p]| {
            (n != p).then(|| {
                [
                    P::always(P::not(premature(n, p))),
                    P::always(time_decreases(n, p)),
                ]
            })
        })
        .flatten();

    let liveness = pairs
        .iter()
        .copied()
        .filter_map(|[n, p]| (n != p).then(|| P::always(P::eventually(ready(n, p)))));

    let mut predicates = vec![];
    predicates.extend(safety);
    predicates.extend(liveness);

    let display_predicates = predicates.iter().map(|p| format!("{p:?}")).join("\n");
    let checker = machine.checked().with_predicates(predicates);
    let initial = checker.initial(initial);
    let result = traverse_checked(
        checker,
        initial,
        TraversalConfig {
            max_depth: None,
            trace_every: Some(25_000),
            ..Default::default()
        },
        |s| {
            let view = s
                .nodes
                .into_iter()
                .map(|(n, ns)| {
                    let ns: Vec<_> = ns
                        .peers
                        .into_iter()
                        .map(|(p, peer)| {
                            (
                                p,
                                peer.timer,
                                match peer.phase {
                                    PeerPhase::Ready => 1,
                                    PeerPhase::Active => 2,
                                    PeerPhase::Closed(outcome) => 3 + outcome.ticks(),
                                },
                            )
                        })
                        .collect();
                    (n, ns)
                })
                .collect_vec();
            Some(view)
        },
    );

    match result {
        Ok(report) => println!("Complete. Report:\n{report:?}"),
        Err(err) => {
            eprintln!("Actions: {:#?}", err.path);
            eprintln!("Error: {}", err.error);
            panic!("properties failed");
        }
    }

    println!("properties satisfied:\n\n{}\n", display_predicates);
}
