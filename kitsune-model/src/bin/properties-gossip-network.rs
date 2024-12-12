use exhaustive::*;

use itertools::Itertools;
use kitsune_model::gossip::gossip_network::*;
use kitsune_model::gossip::gossip_node::*;
use polestar::logic::Pair;
use polestar::logic::PropRegistry;
use polestar::logic::Propositions;
use polestar::model_checker::ModelChecker;
use polestar::prelude::*;
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

    #[derive(Debug, Clone, Copy, derive_more::Display)]
    #[display("Prop({:?})", _0)]
    enum Prop {
        Premature(N, N),
        TimeDecreases(N, N),
        Ready(N, N),
    }

    impl Propositions<Prop> for Pair<GossipState<N>> {
        fn eval(&self, prop: &Prop) -> bool {
            let (s0, s1) = self;
            match *prop {
                Prop::Ready(n, p) => {
                    assert_ne!(n, p);
                    peer_focus(s0, n, p).phase == PeerPhase::Ready
                }
                Prop::TimeDecreases(n, p) => {
                    assert_ne!(n, p);
                    let a = peer_focus(s0, n, p);
                    let b = peer_focus(s1, n, p);
                    a.phase != b.phase || b.timer <= a.timer
                }
                Prop::Premature(n, p) => {
                    assert_ne!(n, p);
                    let a = peer_focus(s0, n, p);
                    let b = peer_focus(s1, n, p);
                    match (a.phase, b.phase) {
                        (PeerPhase::Closed(GossipOutcome::Failure(_)), PeerPhase::Active) => true,
                        (PeerPhase::Closed(GossipOutcome::Success(_)), PeerPhase::Active) => true,
                        (
                            PeerPhase::Active,
                            PeerPhase::Closed(GossipOutcome::Failure(FailureReason::Timeout)),
                        ) => a.timer > 0,
                        _ => false,
                    }
                }
            }
        }
    }

    let mut propmap = PropRegistry::empty();

    let predicates = pairs
        .iter()
        .copied()
        .filter_map(|[n, p]| {
            (n != p).then(|| {
                let ready = propmap.add(Prop::Ready(n, p)).unwrap();
                let premature = propmap.add(Prop::Premature(n, p)).unwrap();
                let time_decreases = propmap.add(Prop::TimeDecreases(n, p)).unwrap();
                [
                    format!("G !{premature}"),
                    format!("G {time_decreases}"),
                    format!("G F {ready}"),
                ]
            })
        })
        .flatten()
        .collect_vec();

    let display_predicates = predicates.iter().map(|p| format!("{p:?}")).join("\n");
    let ltl = predicates.into_iter().join(" && ");

    let checker = ModelChecker::new(machine, propmap, &ltl);

    let result = checker.check_mapped(initial, |s| {
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
    });

    result.expect("model check failed");

    println!("properties satisfied:\n\n{}\n", display_predicates);
}
