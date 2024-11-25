use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt::Debug,
};

use anyhow::{anyhow, bail};
use derive_more::derive::Deref;
use exhaustive::Exhaustive;
use itertools::Itertools;
use polestar::{id::Id, prelude::*, util::first};

use crate::op_single::{OpAction, OpPhase, OpSingleMachine, ValidationType as VT};

/*
                                    █████       ███
                                   ░░███       ░░░
 █████████████    ██████    ██████  ░███████   ████  ████████    ██████
░░███░░███░░███  ░░░░░███  ███░░███ ░███░░███ ░░███ ░░███░░███  ███░░███
 ░███ ░███ ░███   ███████ ░███ ░░░  ░███ ░███  ░███  ░███ ░███ ░███████
 ░███ ░███ ░███  ███░░███ ░███  ███ ░███ ░███  ░███  ░███ ░███ ░███░░░
 █████░███ █████░░████████░░██████  ████ █████ █████ ████ █████░░██████
░░░░░ ░░░ ░░░░░  ░░░░░░░░  ░░░░░░  ░░░░ ░░░░░ ░░░░░ ░░░░ ░░░░░  ░░░░░░
*/

/// Machine that tracks the state of an op and all its dependencies
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OpFamilyMachine<O: Id> {
    pub root: O,
    /// All ops covered, including the root
    pub deps: BTreeSet<O>,
}

impl<O: Id> Machine for OpFamilyMachine<O> {
    type State = OpFamilyState<O>;
    type Action = (O, OpFamilyAction<O>);
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(
        &self,
        mut states: Self::State,
        (target, action): Self::Action,
    ) -> TransitionResult<Self> {
        use OpFamilyAction as E;
        use OpFamilyPhase as S;
        use OpPhase::*;

        if !self.deps.contains(&target) {
            bail!("{target:?} not covered");
        }

        if let E::Await(_, dep) = action {
            if dep == target {
                bail!("An op can't depend on itself")
            }
            if dep == self.root {
                bail!("The focus op can't be depended on")
            }
        }

        let state = states
            .remove(&target)
            .ok_or_else(|| anyhow!("no op {:?}", target))?;

        let next = match (state, action) {
            // Normal op transitions can follow the OpSingle model
            (S::Op(s), E::Op(e)) => S::Op(OpSingleMachine.transition_(s, e)?),

            // Transitions to the Awaiting state
            (S::Op(s), E::Await(vt, dep_id)) => match (vt, s) {
                (VT::Sys, Stored) => S::Awaiting(VT::Sys, dep_id),
                (VT::App, Validated(VT::Sys)) => S::Awaiting(VT::App, dep_id),
                _ => bail!("invalid transition to Awaiting: {state:?} -> {action:?}"),
            },

            // Transitions out of the Awaiting state
            (S::Awaiting(vt, dep_id), E::Op(a)) => match (vt, a) {
                (VT::Sys, OpAction::Validate(VT::Sys)) | (VT::App, OpAction::Validate(VT::App)) => {
                    let dep = states.get(&dep_id).ok_or(anyhow!("no dep {:?}", dep_id))?;
                    if matches!(dep, S::Op(Integrated)) {
                        S::Op(Validated(vt))
                    } else if dep.is_definitely_invalid() {
                        // TODO: can holochain do better here? Would this be a case for Abandoned?
                        state
                    } else {
                        bail!(
                            "attempted to validate op still awaiting dep: {state:?} -> {action:?}"
                        )
                    }
                }
                _ => bail!("invalid transition out of Awaiting: {state:?} -> {action:?}"),
            },

            (state, action) => bail!("invalid transition: {state:?} -> {action:?}"),
            // // Transitions to the Awaiting state (alternate syntax)
            // (S::Op(Pending), E::Await(VT::Sys, dep)) if dep != target => S::Awaiting(VT::Sys, dep),
            // (S::Op(Validated(VT::Sys)), E::Await(VT::App, dep)) if dep != target => {
            //     S::Awaiting(VT::App, dep)
            // }
        };

        states.insert(target, next);

        if detect_loop(&states, target) {
            bail!("this would create a dependency loop: {state:?} -> {action:?}");
        }
        Ok((states, ()))
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        state.values().all(|s| {
            matches!(
                s,
                OpFamilyPhase::Op(OpPhase::Integrated | OpPhase::Rejected)
            )
        })
    }
}

impl<O: Id> OpFamilyMachine<O> {
    pub fn new(root: O, deps: impl IntoIterator<Item = O>) -> Self {
        Self {
            root,
            deps: deps.into_iter().collect(),
        }
    }

    pub fn initial(&self, ids: impl IntoIterator<Item = O>) -> OpFamilyState<O> {
        OpFamilyState::new(ids)
    }
}

/*
          █████               █████
         ░░███               ░░███
  █████  ███████    ██████   ███████    ██████
 ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
░░█████   ░███      ███████   ░███    ░███████
 ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
 ██████   ░░█████ ░░████████  ░░█████ ░░██████
░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░



*/

#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::Deref, derive_more::DerefMut)]
pub struct OpFamilyState<O: Id>(BTreeMap<O, OpFamilyPhase<O>>);

impl<O: Id> OpFamilyState<O> {
    pub fn new(ids: impl IntoIterator<Item = O>) -> Self {
        Self(
            ids.into_iter()
                .map(|id| (id, OpFamilyPhase::default()))
                .collect(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display, derive_more::TryUnwrap)]
pub enum OpFamilyPhase<O: Id, Phase = OpPhase> {
    Op(Phase),
    #[display("Awaiting({}, {})", _0, _1)]
    Awaiting(VT, O),
}

impl<O: Id> Default for OpFamilyPhase<O> {
    fn default() -> Self {
        Self::Op(Default::default())
    }
}

impl<O: Id> OpFamilyPhase<O> {
    pub fn is_definitely_valid(&self) -> bool {
        matches!(
            self,
            OpFamilyPhase::Op(OpPhase::Validated(VT::App)) | OpFamilyPhase::Op(OpPhase::Integrated)
        )
    }

    pub fn is_definitely_invalid(&self) -> bool {
        matches!(self, OpFamilyPhase::Op(OpPhase::Rejected))
    }
}

/*
                     █████     ███
                    ░░███     ░░░
  ██████    ██████  ███████   ████   ██████  ████████
 ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
  ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
 ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
░░████████░░██████   ░░█████  █████░░██████  ████ █████
 ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░



*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::From, Exhaustive)]
pub enum OpFamilyAction<O: Id> {
    #[from]
    Op(OpAction),
    /// Await these ops
    Await(VT, O),
}

/// Given a btreemap of ops to their dependencies, detect if there are any loops
fn detect_loop<O: Id>(state: &BTreeMap<O, OpFamilyPhase<O>>, mut id: O) -> bool {
    let mut visited = HashSet::new();
    visited.insert(id);
    while let Some((_vt, dep)) = state.get(&id).and_then(|s| s.try_unwrap_awaiting().ok()) {
        if !visited.insert(dep) {
            return true;
        }
        id = dep;
    }
    false
}

/*
  █████                      █████
 ░░███                      ░░███
 ███████    ██████   █████  ███████    █████
░░░███░    ███░░███ ███░░  ░░░███░    ███░░
  ░███    ░███████ ░░█████   ░███    ░░█████
  ░███ ███░███░░░   ░░░░███  ░███ ███ ░░░░███
  ░░█████ ░░██████  ██████   ░░█████  ██████
   ░░░░░   ░░░░░░  ░░░░░░     ░░░░░  ░░░░░░



*/

#[derive(Clone, PartialEq, Eq, Hash, derive_more::From)]
pub struct OpFamilyStatePretty<I: Id>(pub OpFamilyState<I>);

impl<I: Id> Debug for OpFamilyStatePretty<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (id, state) in self.0.iter() {
            writeln!(f, "{id} = {state}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use polestar::{
        diagram::exhaustive::write_dot_state_diagram_mapped, id::IdU8, machine::checked::Predicate,
        traversal::traverse_checked,
    };

    #[test]
    fn op_family_properties() {
        use Predicate as P;

        const N: usize = 3;
        type O = IdU8<N>;
        let o = O::all_values();

        let awaiting = |a, b: O| {
            P::atom(format!("{a} awaits {b}"), move |s: &OpFamilyState<O>| {
                s.get(&a)
                    .map(|p| matches!(p, OpFamilyPhase::Awaiting(_, x) if *x == b))
                    .unwrap_or(false)
            })
        };

        let integrated = |a| {
            P::atom(format!("{a} integrated"), move |s: &OpFamilyState<O>| {
                s.get(&a)
                    .map(|p| matches!(p, OpFamilyPhase::Op(OpPhase::Integrated)))
                    .unwrap_or(false)
            })
        };

        let machine: OpFamilyMachine<O> = OpFamilyMachine::new(o[0], o);

        let predicates = (0..N).map(O::new).flat_map(|a| {
            (0..N).map(O::new).flat_map(move |b| {
                [
                    P::always(awaiting(a, b).implies(P::not(awaiting(b, a)))),
                    P::always(
                        awaiting(a, b).implies(P::always(integrated(a).implies(integrated(b)))),
                    ),
                ]
            })
        });

        let checker = machine.clone().checked().with_predicates(predicates);

        let initial = checker.initial(machine.initial(o));

        if let Err(err) = traverse_checked(&checker, initial) {
            eprintln!("{:#?}", err.path);
            eprintln!("{}", err.error);
            panic!("properties failed");
        }

        println!("{:#?}", checker);
    }

    #[test]
    #[ignore = "diagram"]
    fn test_op_family_diagram() {
        use polestar::diagram::exhaustive::DiagramConfig;

        type O = IdU8<2>;
        let o = O::all_values();

        // Create an instance of OpMachine with 1 dependency
        let machine: OpFamilyMachine<O> = OpFamilyMachine::new(o[0], o);

        let initial = OpFamilyState::new(o);

        write_dot_state_diagram_mapped(
            "op-family.dot",
            machine,
            initial,
            &DiagramConfig {
                ..Default::default()
            },
            |state| OpFamilyStatePretty(state),
            |action| action,
        );
    }

    #[test]
    #[ignore = "wrong"]
    fn test_all_pairs() {
        /// Supposed to be a tree of dependencies, but doesn't work
        #[derive(Deref, Clone, derive_more::From)]
        struct OpDeps<O: Id>(Vec<(O, OpDeps<O>)>);

        impl<O: Id> OpDeps<O> {
            pub fn all_pairs(&self) -> HashSet<(O, O)> {
                self.iter()
                    .flat_map(|(x, xs)| {
                        xs.iter().flat_map(|(y, ys)| {
                            ys.all_pairs().into_iter().chain(std::iter::once((*x, *y)))
                        })
                    })
                    .collect()
                // .map(|(x, y)| (x.clone(), y.clone()))
            }
        }

        type O = IdU8<4>;
        let o = O::all_values();

        let deps = OpDeps(vec![(
            o[0],
            vec![(
                o[1],
                vec![(o[2], vec![].into()), (o[3], vec![].into())].into(),
            )]
            .into(),
        )]);
        assert_eq!(
            deps.all_pairs(),
            [(o[0], o[1]), (o[1], o[2]), (o[1], o[3])].into()
        );

        let deps = OpDeps(vec![(
            o[0],
            vec![(
                o[1],
                vec![(o[2], vec![(o[3], vec![].into())].into())].into(),
            )]
            .into(),
        )]);
        assert_eq!(deps.all_pairs(), [(o[0], o[1]), (o[1], o[2])].into());
    }

    #[test]
    fn test_loop() {
        type O = IdU8<3>;
        let o = O::all_values();

        let v = VT::Sys;

        let state: BTreeMap<_, _> = [
            (o[0], OpFamilyPhase::Awaiting(v, o[1])),
            (o[1], OpFamilyPhase::Awaiting(v, o[2])),
            (o[2], OpFamilyPhase::Op(OpPhase::Stored)),
        ]
        .into_iter()
        .collect();
        assert!(!detect_loop(&state, o[0]));

        let state: BTreeMap<_, _> = [
            (o[0], OpFamilyPhase::Awaiting(v, o[1])),
            (o[1], OpFamilyPhase::Awaiting(v, o[2])),
            (o[2], OpFamilyPhase::Awaiting(v, o[0])),
        ]
        .into_iter()
        .collect();
        assert!(detect_loop(&state, o[0]));
    }
}
