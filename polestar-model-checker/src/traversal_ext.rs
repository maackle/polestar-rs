//! Extension traits which add model checking to a [`Traversal`].

use std::{fmt::Debug, hash::Hash, sync::Arc};

use exhaustive::Exhaustive;
use itertools::Itertools;
use polestar_core::Machine;
use polestar_traversal::{Traversal, TraversalReport, traversal::traverse};

use crate::logic::{EvaluatePropositions, PropositionMapping, Transition};
use crate::model_checker::{
    ModelChecker, ModelCheckerError, ModelCheckerState, ModelCheckerTransitionError,
};

/// Adds [`TraversalSpecExt::specced`] to any [`Traversal`].
pub trait TraversalSpecExt<M, S, A>
where
    M: Machine,
    S: 'static + Clone + Debug + Eq + Hash,
    A: 'static + Clone + Debug,
    M::State: 'static + Clone + Debug + Eq + Hash,
    M::Action: 'static + Clone + Debug,
    M::Error: 'static,
{
    /// Add a the specificiation to this traversal.
    /// This is the first step in performing model checking.
    ///
    /// This causes a Buchi automaton to be built from the specification,
    /// which adds additional guards to the state machine. It also sets the
    /// Traversal with the appropriate settings for model checking.
    fn specced<P>(
        self,
        props: P,
        ltl: &str,
    ) -> anyhow::Result<Traversal<ModelChecker<M, P>, ModelCheckerState<S, M::Action>, A>>
    where
        P: PropositionMapping + Send + Sync + 'static,
        Transition<M>: EvaluatePropositions<P::Proposition>;
}

impl<M, S, A> TraversalSpecExt<M, S, A> for Traversal<M, S, A>
where
    M: Machine,
    S: 'static + Clone + Debug + Eq + Hash,
    A: 'static + Clone + Debug,
    M::State: 'static + Clone + Debug + Eq + Hash,
    M::Action: 'static + Clone + Debug,
    M::Error: 'static,
{
    fn specced<P>(
        self,
        props: P,
        ltl: &str,
    ) -> anyhow::Result<Traversal<ModelChecker<M, P>, ModelCheckerState<S, M::Action>, A>>
    where
        P: PropositionMapping + Send + Sync + 'static,
        Transition<M>: EvaluatePropositions<P::Proposition>,
    {
        let machine = ModelChecker::from_ltl(self.machine, props, ltl)?;
        let initial = self
            .initial
            .into_iter()
            .map(|s| machine.initial(s))
            .collect();
        let visitor = self.visitor;
        let map_state = self.map_state;
        let map_action = self.map_action;
        Ok(Traversal {
            machine,
            initial,
            max_depth: self.max_depth,
            trace_every: self.trace_every,
            trace_errors: self.trace_errors,
            ignore_loopbacks: self.ignore_loopbacks,
            visitor: Arc::new(move |s, visit| {
                visitor(&*s, visit).map_err(ModelCheckerTransitionError::MachineError)
            }),
            is_fatal_error: Arc::new(|e| {
                !matches!(e, ModelCheckerTransitionError::MachineError(_))
            }),
            map_state: Arc::new(move |s| s.map_state(|ss| (map_state)(ss))),
            map_action: Arc::new(move |a| (map_action)(a)),
        })
    }
}

/// Adds [`TraversalModelCheckExt::model_check`] to a [`Traversal`] on which
/// [`TraversalSpecExt::specced`] has been called.
pub trait TraversalModelCheckExt<M: Machine> {
    /// Do a model check on a traversal on which [`TraversalSpecExt::specced`] has been called.
    /// This returns a report if the model check succeeds, or any errors if it fails.
    ///
    /// For a more easily readable report, see [`TraversalModelCheckExt::model_check_report`].
    fn model_check(self) -> Result<TraversalReport, ModelCheckerError<M>>;

    /// Performs a model check, and prints a handy report to the console.
    /// This can be unwrapped to panic on error.
    fn model_check_report(self) -> Result<(), String>;
}

impl<M, S, A, P> TraversalModelCheckExt<M>
    for Traversal<ModelChecker<M, P>, ModelCheckerState<S, M::Action>, A>
where
    M: Machine + Send + Sync + 'static,
    M::State: Clone + Debug + Eq + Hash + Send + Sync + 'static,
    S: Clone + Debug + Eq + Hash + Send + Sync + 'static,
    M::Action: Clone + Debug + Eq + Hash + Exhaustive + Send + Sync + 'static,
    A: Clone + Debug + Eq + Hash + Exhaustive + Send + Sync + 'static,
    M::Error: Debug + Send + Sync + 'static,
    P: PropositionMapping + Send + Sync + 'static,
    Transition<M>: EvaluatePropositions<P::Proposition>,
{
    fn model_check(self) -> Result<TraversalReport, ModelCheckerError<M>> {
        match traverse(self, true, false) {
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

    fn model_check_report(self) -> Result<(), String> {
        match self.model_check() {
            Ok(report) => {
                println!("{report:#?}");
                Ok(())
            }
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
                Err("model checker error".into())
            }
        }
    }
}
