use exhaustive::*;

use itertools::Itertools;
use kitsune_model::gossip::gossip_node::*;
use polestar::machine::checked::Predicate as P;
use polestar::prelude::*;
use polestar::traversal::traverse_checked;
use polestar::traversal::TraversalConfig;
use tracing::Level;

fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    type N = UpTo<2>;

    let machine = NodeMachine::<N>::new();

    let initial = machine.initial();

    let premature = |n: N| {
        P::atom2(
            format!("early({n})"),
            move |a: &NodeState<N>, b: &NodeState<N>| {
                let a = a.peers.get(&n).unwrap();
                let b = b.peers.get(&n).unwrap();
                match (a.phase, b.phase) {
                    (PeerPhase::Closed(GossipOutcome::Failure(_)), PeerPhase::Active) => true,
                    (PeerPhase::Closed(GossipOutcome::Success(_)), PeerPhase::Active) => true,
                    (
                        PeerPhase::Active,
                        PeerPhase::Closed(GossipOutcome::Failure(FailureReason::Timeout)),
                    ) => a.timer > 0,
                    _ => false,
                }
            },
        )
    };

    let closed_time_decreases = |n: N| {
        P::atom2(
            format!("closed_time_decreases({n})"),
            move |a: &NodeState<N>, b: &NodeState<N>| {
                let a = a.peers.get(&n).unwrap();
                let b = b.peers.get(&n).unwrap();
                if matches!(a.phase, PeerPhase::Closed(_))
                    && matches!(b.phase, PeerPhase::Closed(_))
                {
                    a.timer >= b.timer
                } else {
                    true
                }
            },
        )
    };

    let ready = |n: N| {
        P::atom(format!("ready({n})"), move |s: &NodeState<N>| {
            s.peers.get(&n).unwrap().phase == PeerPhase::Ready
        })
    };

    let safety = N::iter_exhaustive(None).flat_map(|n| {
        [
            P::always(P::not(premature(n))),
            P::always(closed_time_decreases(n)),
        ]
    });

    let liveness = N::iter_exhaustive(None).flat_map(|n| [P::always(P::eventually(ready(n)))]);

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
            trace_every: 25_000,
            ..Default::default()
        },
        |s: NodeState<N>| {
            let view = s
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
