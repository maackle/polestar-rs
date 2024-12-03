use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Display},
};

use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use polestar::{ext::MapExt, id::Id, prelude::*};
use serde::{Deserialize, Serialize};

use super::gossip_node::*;

/*                   █████     ███
                    ░░███     ░░░
  ██████    ██████  ███████   ████   ██████  ████████
 ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
  ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
 ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
░░████████░░██████   ░░█████  █████░░██████  ████ █████
 ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░   */

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Exhaustive, Serialize, Deserialize)]
pub struct GossipAction<N: Id>(N, NodeAction<N>);

/*        █████               █████
         ░░███               ░░███
  █████  ███████    ██████   ███████    ██████
 ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
░░█████   ░███      ███████   ░███    ░███████
 ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
 ██████   ░░█████ ░░████████  ░░█████ ░░██████
░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░  */

/// The panoptic state of the whole network
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, derive_more::Constructor)]
pub struct GossipState<N: Id> {
    nodes: BTreeMap<N, NodeState<N>>,
}

/*                                  █████       ███
                                   ░░███       ░░░
 █████████████    ██████    ██████  ░███████   ████  ████████    ██████
░░███░░███░░███  ░░░░░███  ███░░███ ░███░░███ ░░███ ░░███░░███  ███░░███
 ░███ ░███ ░███   ███████ ░███ ░░░  ░███ ░███  ░███  ░███ ░███ ░███████
 ░███ ░███ ░███  ███░░███ ░███  ███ ░███ ░███  ░███  ░███ ░███ ░███░░░
 █████░███ █████░░████████░░██████  ████ █████ █████ ████ █████░░██████
░░░░░ ░░░ ░░░░░  ░░░░░░░░  ░░░░░░  ░░░░ ░░░░░ ░░░░░ ░░░░ ░░░░░  ░░░░░░  */

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GossipMachine<N: Id> {
    node_machine: NodeMachine<N>,
}

impl<N: Id> Machine for GossipMachine<N> {
    type State = GossipState<N>;
    type Action = GossipAction<N>;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(
        &self,
        mut state: Self::State,
        GossipAction(node, action): Self::Action,
    ) -> TransitionResult<Self> {
        // TODO: probably need to be smarter about what actually happens here.
        match action {
            NodeAction::AddPeer(peer) if node == peer => {
                bail!("node cannot add itself as a peer");
            }
            action => state
                .nodes
                .owned_update(node, |_, node| self.node_machine.transition(node, action))?,
        };
        Ok((state, ()))
    }

    fn is_terminal(&self, s: &Self::State) -> bool {
        false
    }
}

impl<N: Id> GossipMachine<N> {
    pub fn new() -> Self {
        Self {
            node_machine: NodeMachine::new(),
        }
    }

    pub fn initial(&self) -> GossipState<N>
    where
        N: Exhaustive,
    {
        GossipState::new(
            N::iter_exhaustive(None)
                .map(|n| {
                    (
                        n,
                        NodeState::new(N::iter_exhaustive(None).filter(|p| p != &n)),
                    )
                })
                .collect(),
        )
    }
}

/*█████                      █████
 ░░███                      ░░███
 ███████    ██████   █████  ███████    █████
░░░███░    ███░░███ ███░░  ░░░███░    ███░░
  ░███    ░███████ ░░█████   ░███    ░░█████
  ░███ ███░███░░░   ░░░░███  ░███ ███ ░░░░███
  ░░█████ ░░██████  ██████   ░░█████  ██████
   ░░░░░   ░░░░░░  ░░░░░░     ░░░░░  ░░░░░░*/

#[cfg(test)]
mod tests {

    use super::*;
    use itertools::Itertools;
    use polestar::{
        diagram::exhaustive::*,
        id::{IdUnit, UpTo},
        machine::checked::Predicate as P,
        traversal::traverse_checked,
    };

    #[test]
    fn properties() {
        type N = UpTo<2>;

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

    #[test]
    #[ignore = "diagram"]
    fn diagram() {
        // With 3 nodes, scheduled:                nodes=13824, edges=107136, finished in 52.91s
        // with 3 nodes, unscheduled with no tick: nodes=4096,  edges=18432,  finished in 68.64s
        type N = UpTo<2>;

        let machine = GossipMachine::<N>::new();
        let state = machine.initial();

        write_dot_state_diagram_mapped(
            "gossip-network.dot",
            machine,
            state,
            &DiagramConfig {
                max_depth: None,
                ..Default::default()
            },
            |state| {
                Some({
                    let lines = state
                        .nodes
                        .into_iter()
                        .map(|(n, s)| {
                            let s = NodeStateUnscheduled::from(s);
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
            |GossipAction(node, action)| Some(format!("{node}: {action}")),
            // |GossipAction(node, action)| {
            //     (!matches!(action, NodeAction::Tick)).then_some(format!("{node}: {action}"))
            // },
        );
    }
}
