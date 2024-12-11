pub mod parser;

use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};

use anyhow::anyhow;
use im::Vector;
use itertools::Itertools;
use parser::*;

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
        state: Arc<BuchiState>,
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

        // // If the buchi state is empty, it is all-accepting, and we never have to check it again.
        // if buchi.is_empty() {
        //     return self
        //         .machine
        //         .transition(state, action)
        //         .map(|(next, fx)| (PromelaState { state: next, buchi }, fx))
        //         .map_err(BuchiError::MachineError);
        // }

        match &*buchi {
            BuchiState::AcceptAll => {
                let (next, fx) = self
                    .machine
                    .transition(state, action)
                    .map_err(BuchiError::MachineError)?;
                Ok((PromelaState { state: next, buchi }, fx))
            }
            BuchiState::Conditional { predicates, .. } => {
                if let Some((_, next_state_name)) = predicates
                    .iter()
                    .find(|(ltl, _)| dbg!(ltl).eval(&state.state))
                {
                    let (next, fx) = self
                        .machine
                        .transition(state, action)
                        .map_err(BuchiError::MachineError)?;
                    let buchi_next = self
                        .buchi
                        .states
                        .get(next_state_name)
                        .ok_or_else(|| {
                            BuchiError::Internal(anyhow!(
                                "no buchi state named '{next_state_name}'. This is a polestar bug."
                            ))
                        })?
                        .clone();
                    let next = PromelaState {
                        state: next,
                        buchi: buchi_next,
                    };
                    Ok((next, fx))
                } else {
                    Err(BuchiError::LtlError {
                        error: anyhow!("LTL not satisfied"),
                        state: buchi.clone(),
                        path: state.path,
                    })
                }
            }
        }
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
            .filter(|name| name.ends_with("_init"))
            .collect_vec();

        if inits.len() != 1 {
            panic!("expected exactly one initial state, found {inits:?}");
        }

        let buchi_init = self.buchi.states.get(inits[0]).cloned().unwrap();

        PromelaState::new(state, buchi_init)
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
    buchi: Arc<BuchiState>,
}

// XXX: equality and hash ignore path! This is necessary for traversal to work well.
impl<M> PartialEq for PromelaState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
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
        self.state.hash(state);
    }
}

impl<M> PromelaState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    pub fn new(state: M::State, buchi_state: Arc<BuchiState>) -> Self {
        Self {
            state: StorePathState::new(state),
            buchi: buchi_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        id::UpTo,
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
            let next = if group == 0 {
                if bump {
                    state + 1
                } else {
                    state * 4
                }
            } else {
                (group * 4) + ((state + 1) % 4)
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
                "is2" => *self == 2,
                "is3" => *self == 3,
                "is4" => *self == 4,
                "is7" => *self == 7,
                "is8" => *self == 8,
                "is11" => *self == 11,
                "is15" => *self == 15,
                p => unreachable!("can't eval unknown prop '{p}' with state {self}"),
            }
        }
    }

    #[derive(Debug, derive_more::Display, Clone, Copy, PartialEq, Eq)]
    #[display("({}, {})", _0, _1)]
    struct Node(u8, bool);

    #[test]
    fn promela_test() {
        // let ltl = "G F (is7 || is11 || is15)";

        let ltl = "( F (is2 && X is8) -> G F is11 )";
        // let ltl = "G( is3 -> G F is15 )";
        // let ltl = "G( is4 -> G F is7  )";

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

        let config = TraversalConfig {
            record_terminals: false,
            trace_every: 1,
            graphing: Some(TraversalGraphingConfig::new(
                |s: &PromelaState<TestMachine2>| s.clone(), //format!("{s:#?}"),
                // |s: &PromelaState<_>| Node(s.state.state, s.buchi.is_accepting()),
                |e| *e,
            )),
            ..Default::default()
        }
        .with_fatal_error(|e| !matches!(e, BuchiError::MachineError(_)));

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
                |_, n| n.iter().map(|n| format!("{n:?}")).collect_vec().join("\n"),
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
                graphing: Some(TraversalGraphingConfig::new(|s| *s, |e| *e)),
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
