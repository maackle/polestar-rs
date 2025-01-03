use std::sync::Arc;

use exhaustive::Exhaustive;
use itertools::Itertools;

use crate::traversal::{traverse, TraversalConfig, TraversalGraphingConfig, TraversalReport};

use super::*;

impl<M, P> ModelChecker<M, P>
where
    M: Machine + Send + Sync + 'static,
    M::State: Clone + Debug + Eq + Hash + Send + Sync + 'static,
    M::Action: Clone + Debug + Eq + Hash + Exhaustive + Send + Sync + 'static,
    M::Error: Debug + Send + Sync + 'static,
    P: PropMapping + Send + Sync + 'static,
    Transition<M>: Propositions<P::Prop>,
{
    pub fn check(
        self: Arc<Self>,
        initial: M::State,
    ) -> Result<TraversalReport, ModelCheckerError<M>> {
        self.check_mapped(initial, Some)
    }

    pub fn check_mapped<S>(
        self: Arc<Self>,
        initial: M::State,
        map_state: impl Fn(M::State) -> Option<S> + Send + Sync + 'static,
    ) -> Result<TraversalReport, ModelCheckerError<M>>
    where
        S: Clone + Debug + Eq + Hash + Send + Sync + 'static,
    {
        let config = TraversalConfig::builder()
            .record_terminals(false)
            .trace_every(100_000)
            .graphing(TraversalGraphingConfig::default())
            .is_fatal_error(|e| !matches!(e, ModelCheckerTransitionError::MachineError(_)))
            .build();

        let initial = self.initial(initial);

        // Replace just the innermost state type, keeping the rest the same
        let map_state =
            move |mcs: ModelCheckerState<M::State, M::Action>| mcs.map_state(&map_state);

        match traverse(self, initial, config, map_state) {
            Ok((report, graph, _)) => {
                let condensed = petgraph::algo::condensation(graph.unwrap(), true);

                let leaves = condensed.node_indices().filter(|n| {
                    let outgoing = condensed
                        .neighbors_directed(*n, petgraph::Direction::Outgoing)
                        .count();
                    outgoing == 0
                });

                for index in leaves {
                    let scc = condensed.node_weight(index).unwrap();
                    let accepting = scc.iter().any(|n| n.buchi.is_accepting());
                    if !accepting {
                        let mut paths = scc.iter().map(|n| n.pathstate.path.clone()).collect_vec();
                        paths.sort_by_key(|p| p.len());
                        return Err(ModelCheckerError::Liveness { paths });
                    }
                }

                Ok(report)
            }
            Err(e) => match e {
                ModelCheckerTransitionError::BuchiError(e) => Err(ModelCheckerError::Safety {
                    path: e.path,
                    states: e.states,
                }),
                ModelCheckerTransitionError::MachineError(e) => {
                    unreachable!("{e:?}");
                }
            },
        }
    }
}

pub fn model_checker_report<M: Machine>(result: Result<TraversalReport, ModelCheckerError<M>>)
where
    M::State: Debug,
    M::Action: Debug + Clone,
{
    match result {
        Ok(report) => println!("{report:#?}"),
        Err(e) => {
            match e {
                ModelCheckerError::Safety {
                    path,
                    states: (cur, next),
                } => {
                    println!("Model checker safety check failed.");
                    println!();
                    println!("path: {path:#?}");
                    println!();
                    println!("last two states:");
                    println!();
                    println!("failing state: {cur:#?}");
                    println!("next state: {next:#?}");
                }
                ModelCheckerError::Liveness { paths } => {
                    println!("Model checker liveness check failed.");
                    println!();
                    println!("paths: {paths:#?}");
                }
            }
            panic!("model checker error");
        }
    }
}

#[derive(derive_bounded::Debug)]
#[bounded_to(M::State, M::Action)]
pub enum ModelCheckerError<M: Machine>
where
    M::Action: Clone,
{
    Safety {
        path: im::Vector<M::Action>,
        states: (M::State, M::State),
    },
    Liveness {
        paths: Vec<im::Vector<M::Action>>,
    },
}
