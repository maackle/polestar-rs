pub mod parser;

use std::{
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    hash::Hash,
    sync::Arc,
};

use anyhow::anyhow;
use im::Vector;
use itertools::Itertools;
use parser::*;

use crate::logic::Propositions;

use super::{
    store_path::{StorePathMachine, StorePathState},
    Machine, TransitionResult,
};

#[derive(derive_bounded::Debug)]
#[bounded_to(M::Error, M::Action)]
pub enum BuchiError<M>
where
    M: Machine,
    M::Action: Debug + Clone,
    M::Error: Debug,
{
    Internal(anyhow::Error),
    LtlError {
        error: anyhow::Error,
        path: Vector<M::Action>,
        // state: Arc<BuchiPaths>,
    },
    MachineError(M::Error),
}

#[derive(Clone)]
pub struct PromelaMachine<M>
where
    M: Machine,
{
    buchi: PromelaBuchi,
    machine: StorePathMachine<M>,
}

/*                                   █████       ███
                                   ░░███       ░░░
 █████████████    ██████    ██████  ░███████   ████  ████████    ██████
░░███░░███░░███  ░░░░░███  ███░░███ ░███░░███ ░░███ ░░███░░███  ███░░███
 ░███ ░███ ░███   ███████ ░███ ░░░  ░███ ░███  ░███  ░███ ░███ ░███████
 ░███ ░███ ░███  ███░░███ ░███  ███ ░███ ░███  ░███  ░███ ░███ ░███░░░
 █████░███ █████░░████████░░██████  ████ █████ █████ ████ █████░░██████
░░░░░ ░░░ ░░░░░  ░░░░░░░░  ░░░░░░  ░░░░ ░░░░░ ░░░░░ ░░░░ ░░░░░  ░░░░░░   */

impl<M> Machine for PromelaMachine<M>
where
    M: Machine,
    M::State: Propositions + Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    type State = PromelaState<M>;
    type Action = M::Action;
    type Error = BuchiError<M>;
    type Fx = M::Fx;

    fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        let PromelaState { state, buchi } = state;

        (&state.state);
        (buchi.0.len());
        let buchi_next = buchi
            .0
            .into_iter()
            .flat_map(|buchi_name| {
                let buchi_state = self
                    .buchi
                    .states
                    .get(&buchi_name)
                    .expect("no buchi state named '{buchi_name}'. This is a polestar bug.");
                // .ok_or_else(|| {
                //     BuchiError::Internal(anyhow!(
                //         "no buchi state named '{buchi_name}'. This is a polestar bug."
                //     ))
                // })?;
                match &**buchi_state {
                    BuchiState::AcceptAll => todo!(), // vec![Ok((buchi_name, buchi_state))],
                    BuchiState::Conditional { predicates, .. } => {
                        (predicates.len());
                        predicates
                            .iter()
                            .filter_map(|(ltl, name)| ((ltl).eval(&state.state)).then_some(name))
                            .cloned()
                            .collect::<BTreeSet<_>>()
                    }
                }
            })
            .collect::<BTreeSet<_>>();

        if buchi_next.is_empty() {
            return Err(BuchiError::LtlError {
                error: anyhow!("LTL not satisfied"),
                path: state.path,
            });
        }

        let (next, fx) = self
            .machine
            .transition(state, action)
            .map_err(BuchiError::MachineError)?;

        let next = PromelaState {
            state: next,
            buchi: buchi_next.into(),
        };
        Ok((next, fx))
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        self.machine.is_terminal(&state.state)
    }
}

impl<M> PromelaMachine<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    pub fn new(machine: M, ltl: &str) -> Self {
        let buchi = PromelaBuchi::from_ltl(ltl);
        Self {
            buchi,
            machine: StorePathMachine::from(machine),
        }
    }

    pub fn initial(&self, state: M::State) -> PromelaState<M> {
        let inits = self
            .buchi
            .states
            .keys()
            .cloned()
            .filter(|name| name.ends_with("_init"));

        PromelaState::new(state, inits)
    }
}

#[derive(derive_more::Debug, derive_bounded::Clone, derive_more::Deref)]
#[bounded_to(StorePathState<M>)]
pub struct PromelaState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    #[deref]
    state: StorePathState<M>,
    #[debug(skip)]
    buchi: BuchiPaths,
}

// XXX: equality and hash ignore path! This is necessary for traversal to work well.
impl<M> PartialEq for PromelaState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    fn eq(&self, other: &Self) -> bool {
        self.state.state == other.state.state && self.buchi == other.buchi
    }
}

impl<M> Eq for PromelaState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
}

impl<M> Hash for PromelaState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.state.state.hash(state);
        self.buchi.hash(state);
    }
}

impl<M> PromelaState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    pub fn new(state: M::State, buchi_states: impl IntoIterator<Item = StateName>) -> Self {
        Self {
            state: StorePathState::new(state),
            buchi: BuchiPaths(buchi_states.into_iter().collect()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        id::UpTo,
        logic::Propositions,
        traversal::{traverse, TraversalConfig, TraversalGraphingConfig},
    };
    use itertools::Itertools;
    use petgraph::visit::IntoNeighborsDirected;

    use super::*;
    use crate::diagram::exhaustive::*;

    const MODULO: usize = 16;

    #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    struct TestMachine1;

    #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    struct TestMachine2;

    impl Machine for TestMachine1 {
        type State = u8;
        type Action = ();

        fn transition(&self, state: Self::State, (): Self::Action) -> TransitionResult<Self> {
            Ok((state.wrapping_add(2) % MODULO as u8, ()))
        }

        fn is_terminal(&self, _: &Self::State) -> bool {
            false
        }
    }

    impl Machine for TestMachine2 {
        type State = u8;
        type Action = bool;

        fn transition(&self, state: Self::State, bump: Self::Action) -> TransitionResult<Self> {
            let group = state / 4;
            let next = if state < 3 {
                if bump {
                    state + 1
                } else {
                    state * 4
                }
            } else {
                (group * 4) + ((state + 1) % 3)
            };
            Ok((next, ()))
        }

        fn is_terminal(&self, _: &Self::State) -> bool {
            false
        }
    }

    impl Propositions for u8 {
        fn eval(&self, p: &str) -> bool {
            match p {
                "even" => self % 2 == 0,
                "max" => *self == (MODULO - 1) as u8,
                "is1" => *self == 1,
                "is2" => *self == 2,
                "is3" => *self == 3,
                "is4" => *self == 4,
                "is5" => *self == 5,
                "is6" => *self == 6,
                "is7" => *self == 7,
                "is8" => *self == 8,
                "is9" => *self == 9,
                "is11" => *self == 11,
                "is15" => *self == 15,
                p => unreachable!("can't eval unknown prop '{p}' with state {self}"),
            }
        }
    }

    #[derive(Debug, derive_more::Display, Clone, Copy, PartialEq, Eq)]
    #[display("({}, {})", _0, _1)]
    struct Node(u8, bool);

    //  ███████████ ██████████  █████████  ███████████
    // ░█░░░███░░░█░░███░░░░░█ ███░░░░░███░█░░░███░░░█
    // ░   ░███  ░  ░███  █ ░ ░███    ░░░ ░   ░███  ░
    //     ░███     ░██████   ░░█████████     ░███
    //     ░███     ░███░░█    ░░░░░░░░███    ░███
    //     ░███     ░███ ░   █ ███    ░███    ░███
    //     █████    ██████████░░█████████     █████
    //    ░░░░░    ░░░░░░░░░░  ░░░░░░░░░     ░░░░░

    #[test]
    fn promela_test() {
        // true negatives:
        let ltl = "G ( is4 -> X is5 ) ";
        let ltl = "G ( is4 -> X is8 ) ";
        let ltl = "G ( is4 -> F is2 )";
        let ltl = "G ( (F is4 && !(F is2)) || (F is2 && !(F is4)) )";
        let ltl = "G ( is3 -> G F is5 )";

        // true positives:
        let ltl = "G ( (is2 && X is6) -> G F is5)";
        let ltl = "G ( is1 -> (G F is5 || G F is3 || G F is8) )";
        let ltl = "G ( is2 -> F is4 )";

        let machine = PromelaMachine::new(TestMachine2, ltl);
        let initial = machine.initial(1);

        // write_dot_state_diagram_mapped(
        //     "promela-diagram.dot",
        //     machine.clone(),
        //     initial.clone(),
        //     &DiagramConfig {
        //         max_depth: None,
        //         ..Default::default()
        //     },
        //     |s| Some(format!("{s:?}")),
        //     Some,
        // );

        let config = TraversalConfig::builder()
            .record_terminals(false)
            .trace_every(1000)
            .graphing(TraversalGraphingConfig::default())
            .is_fatal_error(|e| !matches!(e, BuchiError::MachineError(_)))
            .visitor(|s: &PromelaState<TestMachine2>, _| {
                println!(
                    "<:> {}: buchi {:?} path {:?}",
                    &s.state.state, &s.buchi, s.state.path
                );
                Ok(())
            })
            .build();

        let (report, graph, _) = traverse(machine, initial, config, Some).unwrap();

        let graph = graph.unwrap();

        {
            let graph = graph.map(|_, n| Node(n.state.state, n.buchi.is_accepting()), |_, e| e);
            crate::diagram::write_dot(
                "promela-verify.dot",
                &graph,
                // &[petgraph::dot::Config::EdgeNoLabel],
                &[],
            );
        }

        dbg!(&report);

        let condensed = petgraph::algo::condensation(graph, true);

        let leaves = condensed.node_indices().filter(|n| {
            let outgoing = condensed
                .neighbors_directed(*n, petgraph::Direction::Outgoing)
                .count();
            outgoing == 0
        });

        {
            let condensed = condensed.map(
                |_, n| {
                    n.iter()
                        .map(|s| {
                            let tup = (s.state.state, &s.buchi);
                            format!("{tup:?}")
                        })
                        .collect_vec()
                        .join("\n")
                },
                |_, e| e,
            );

            crate::diagram::write_dot(
                "promela-verify-condensed.dot",
                &condensed,
                &[petgraph::dot::Config::EdgeNoLabel],
                // &[],
            );
        }

        for index in leaves {
            let scc = condensed.node_weight(index).unwrap();
            let accepting = scc.iter().any(|n| n.buchi.is_accepting());
            if !accepting {
                let mut paths = scc.iter().map(|n| n.state.path.clone()).collect_vec();
                paths.sort();
                dbg!(&paths);
                panic!("non-accepting SCC found");
            }
        }

        //////////////////////////////////////////////////////////////////

        // let scc = petgraph::algo::kosaraju_scc(&graph);

        // dbg!(&scc);

        // for (i, nodes) in scc.iter().enumerate() {
        //     let accepting = nodes.iter().any(|n| graph.node_weight(*n).unwrap().1);
        //     if !accepting {
        //         panic!("non-accepting SCC found");
        //     } else {
        //         println!("SCC {i} accepting");
        //     }
        // }
    }

    #[test]
    #[ignore = "diagram"]
    fn promela_diagram() {
        let (_, graph, _) = traverse(
            TestMachine2,
            1,
            TraversalConfig {
                // record_terminals: true,
                // trace_every: 1,
                graphing: Some(TraversalGraphingConfig::default()),
                ..Default::default()
            },
            // .with_fatal_error(|e| !matches!(e, BuchiError::MachineError(_))),
            Some,
        )
        .unwrap();

        let graph = graph.unwrap();

        crate::diagram::write_dot(
            "promela-traversal.dot",
            &graph,
            // &[petgraph::dot::Config::EdgeNoLabel],
            &[],
        );

        write_dot_state_diagram_mapped(
            "promela-diagram.dot",
            TestMachine2,
            1,
            &DiagramConfig {
                max_depth: None,
                ..Default::default()
            },
            Some,
            Some,
        );
    }
}
