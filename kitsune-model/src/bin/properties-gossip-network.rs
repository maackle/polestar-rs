use exhaustive::*;

use itertools::Itertools;
use kitsune_model::gossip::gossip_network::*;
use kitsune_model::gossip::gossip_node::*;
use polestar::logic::Pair;
use polestar::logic::PropRegistry;
use polestar::logic::Propositions;
use polestar::model_checker::ModelChecker;
use polestar::model_checker::ModelCheckerError;
use polestar::prelude::*;
use polestar::traversal::TraversalGraphingConfig;
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

    #[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
    enum Prop {
        #[display("Premature({}, {})", _0, _1)]
        Premature(N, N),
        #[display("TimeDecreases({}, {})", _0, _1)]
        TimeDecreases(N, N),
        #[display("Ready({}, {})", _0, _1)]
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
                    // TODO: predicates should be able to know about the current Action as well.
                    //       this is a hack in lieu of that.
                    a.phase != b.phase || a.phase == PeerPhase::Active || b.timer <= a.timer
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

    let checker = ModelChecker::new(machine, propmap, &ltl).unwrap();

    {
        let config = polestar::traversal::TraversalConfig::builder()
            .max_depth(1)
            .graphing(Default::default())
            .build();
        let (_, graph, _) = polestar::traversal::traverse(
            checker.clone(),
            checker.initial(initial.clone()),
            config,
            Some,
        )
        .unwrap();

        let mapstate = |state: &GossipState<N>| {
            let lines = state
                .nodes
                .iter()
                .map(|(n, s)| {
                    let s = NodeStateSimple::new(true, &s);
                    format!("{s}")
                        .split('\n')
                        .filter_map(|l| (!l.is_empty()).then_some(format!("{n}↤{l}")))
                        .join("\n")
                })
                .collect_vec()
                .join("\n");
            format!("{lines}\n")
        };

        let graph = graph.unwrap().map(
            |_, n| format!("{}\n{}", n.buchi.is_accepting(), mapstate(&n.state)),
            |_, GossipAction(node, action)| {
                format!("{node}: {}", NodeAction::<N, IdUnit>::from(action.clone()))
            },
        );

        polestar::diagram::write_dot("out.dot", &graph, &[]);
    }

    let result = checker.check(initial);
    // let result = checker.check_mapped(initial, |s| {
    //     let view = s
    //         .nodes
    //         .into_iter()
    //         .map(|(n, ns)| {
    //             let ns: Vec<_> = ns
    //                 .peers
    //                 .into_iter()
    //                 .map(|(p, peer)| {
    //                     (
    //                         p,
    //                         peer.timer,
    //                         match peer.phase {
    //                             PeerPhase::Ready => 1,
    //                             PeerPhase::Active => 2,
    //                             PeerPhase::Closed(outcome) => 3 + outcome.ticks(),
    //                         },
    //                     )
    //                 })
    //                 .collect();
    //             (n, ns)
    //         })
    //         .collect_vec();
    //     Some(view)
    // });

    if let Err(e) = &result {
        match e {
            ModelCheckerError::Safety {
                path,
                states: (cur, next),
            } => {
                let machine = GossipMachine::<N>::new();
                let initial = machine.initial();
                machine
                    .apply_each_action(initial, path.clone(), |action, state| {
                        dbg!(state);
                    })
                    .unwrap();
            }
            ModelCheckerError::Liveness { paths } => {}
        }
    }
    polestar::model_checker::model_checker_report(result);

    println!("properties satisfied:\n\n{}\n", display_predicates);
}
