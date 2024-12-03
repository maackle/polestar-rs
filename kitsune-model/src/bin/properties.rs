use exhaustive::*;

use kitsune_model::gossip::gossip_network::*;
use kitsune_model::gossip::gossip_node::*;
use polestar::machine::checked::Predicate as P;
use polestar::prelude::*;
use polestar::traversal::traverse_checked;

fn main() {
    type N = UpTo<3>;

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

    let ready = |n: N, p: N| {
        assert_ne!(n, p);
        P::atom(format!("{n}_ready_for_{p}"), move |s: &GossipState<N>| {
            s.nodes.get(&n).unwrap().schedule.get_key(&p).unwrap() == &PeerState::Ready
        })
    };

    let liveness = <(N, N)>::iter_exhaustive(None)
        .filter_map(|(n, p)| (n != p).then(|| P::always(P::eventually(ready(n, p)))));

    let mut predicates = vec![];
    predicates.extend(liveness);

    let checker = machine.checked().with_predicates(predicates);
    let initial = checker.initial(initial);
    if let Err(err) = traverse_checked(checker, initial) {
        eprintln!("{:#?}", err.path);
        eprintln!("{}", err.error);
        panic!("properties failed");
    };
}
