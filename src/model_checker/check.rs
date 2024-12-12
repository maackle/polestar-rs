use exhaustive::Exhaustive;
use itertools::Itertools;

use crate::traversal::{traverse, TraversalConfig, TraversalGraphingConfig, TraversalReport};

use super::*;

impl<M, P> ModelChecker<M, P>
where
    M: Machine + Send + Sync + 'static,
    M::State: Clone + Debug + Eq + Hash + Propositions<P> + Send + Sync + 'static,
    M::Action: Clone + Debug + Eq + Hash + Exhaustive + Send + Sync + 'static,
    M::Error: Send + Sync + 'static,
    P: Display + Clone + Send + Sync + 'static,
{
    pub fn check(self, initial: M::State) -> Result<TraversalReport, ModelCheckerError<M>> {
        let config = TraversalConfig::builder()
            .record_terminals(false)
            // .trace_every(1000)
            .graphing(TraversalGraphingConfig::default())
            .is_fatal_error(|e| !matches!(e, ModelCheckerTransitionError::MachineError(_)))
            .build();

        let initial = self.initial(initial);

        match traverse(self, initial, config, Some) {
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
                        let mut paths = scc.iter().map(|n| n.state.path.clone()).collect_vec();
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
