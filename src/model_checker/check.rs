use exhaustive::Exhaustive;
use itertools::Itertools;

use crate::traversal::{traverse, TraversalConfig, TraversalGraphingConfig, TraversalReport};

use super::*;

impl<M, P> ModelChecker<M, P>
where
    M: Machine + Send + Sync + 'static,
    M::State: Clone + Debug + Eq + Hash + Send + Sync + 'static,
    Pair<M::State>: Propositions<P>,
    M::Action: Clone + Debug + Eq + Hash + Exhaustive + Send + Sync + 'static,
    M::Error: Send + Sync + 'static,
    P: Display + Clone + Send + Sync + 'static,
{
    pub fn check(self, initial: M::State) -> Result<TraversalReport, ModelCheckerError<M>> {
        self.check_mapped(initial, Some)
    }

    pub fn check_mapped<S>(
        self,
        initial: M::State,
        map_state: impl Fn(M::State) -> Option<S> + Send + Sync + 'static,
    ) -> Result<TraversalReport, ModelCheckerError<M>>
    where
        S: Clone + Debug + Eq + Hash + Send + Sync + 'static,
    {
        let config = TraversalConfig::builder()
            .record_terminals(false)
            // .trace_every(1000)
            .graphing(TraversalGraphingConfig::default())
            .is_fatal_error(|e| !matches!(e, ModelCheckerTransitionError::MachineError(_)))
            .build();

        let initial = self.initial(initial);

        // Replace just the innermost state type, keeping the rest the same
        let map_state = move |mcs: ModelCheckerState<M::State, M::Action>| {
            let path = mcs.pathstate.path;
            let state = map_state(mcs.pathstate.state)?;
            let pathstate = StorePathState::<S, M::Action> { state, path };
            Some(ModelCheckerState {
                pathstate,
                buchi: mcs.buchi,
            })
        };

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
                ModelCheckerTransitionError::BuchiError(e) => {
                    Err(ModelCheckerError::Safety { path: e.path })
                }
                ModelCheckerTransitionError::MachineError(e) => {
                    unreachable!("{e:?}");
                }
            },
        }
    }
}

#[derive(Debug)]
pub enum ModelCheckerError<M: Machine>
where
    M::Action: Clone,
{
    Safety { path: im::Vector<M::Action> },
    Liveness { paths: Vec<im::Vector<M::Action>> },
}
