use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt::{Debug, Pointer},
};

use anyhow::{anyhow, bail};
use derive_more::derive::Deref;
use exhaustive::Exhaustive;
use itertools::Itertools;
use polestar::{
    id::{Id, IdU8},
    prelude::*,
    util::{first, second},
};
use serde::{Deserialize, Serialize};

use crate::op_single::{OpAction, OpPhase, OpSingleMachine, Outcome, ValidationType as VT};

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
pub struct OpFamilyMachine<O: Id, T: Id> {
    /// All ops covered, including the root
    pub deps: Option<BTreeSet<(O, T)>>,
}

impl<O: Id, T: Id> Machine for OpFamilyMachine<O, T> {
    type State = OpFamilyState<O, T>;
    type Action = ((O, T), OpFamilyAction<O>);
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(
        &self,
        mut states: Self::State,
        (target, action): Self::Action,
    ) -> TransitionResult<Self> {
        use crate::op_single::Outcome as VO;
        use OpFamilyAction as E;
        use OpFamilyPhase as S;
        use OpPhase::*;

        if self
            .deps
            .as_ref()
            .map(|ds| !ds.contains(&target))
            .unwrap_or(false)
        {
            bail!("{target:?} not covered");
        }

        // If deps aren't bounded, add a new state when seen
        if !states.contains_key(&target) {
            states.insert(target, OpFamilyPhase::default());
        }

        if let E::Await(_, dep) = action {
            if dep == target.0 {
                bail!("An op can't depend on itself")
            }

            if self
                .deps
                .as_ref()
                .and_then(|ds| ds.first())
                .map(|d| dep == d.0)
                .unwrap_or(false)
            {
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
                    let dep = states
                        .iter_from(dep_id)
                        .map(|(_, dep)| dep)
                        .find(|dep| matches!(dep, S::Op(Integrated(_))))
                        .ok_or(anyhow!(
                            "attempted to validate op still awaiting dep {:?}",
                            dep_id
                        ))?;

                    match dep {
                        S::Op(Integrated(VO::Accepted)) => S::Op(Validated(vt)),
                        S::Op(Integrated(VO::Rejected)) => {
                            // TODO: can holochain do better here? Would this be a case for Abandoned?
                            state
                        }
                        _ => unreachable!(),
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

        if detect_loop(&states, target.0) {
            bail!("this would create a dependency loop: {state:?} -> {action:?}");
        }
        Ok((states, ()))
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        false
        // state.values().all(|s| {
        //     matches!(
        //         s,
        //         OpFamilyPhase::Op(OpPhase::Integrated | OpPhase::Rejected)
        //     )
        // })
    }
}

impl<O: Id, T: Id> OpFamilyMachine<O, T> {
    pub fn new() -> Self {
        Self { deps: None }
    }

    pub fn new_bounded(deps: impl IntoIterator<Item = (O, T)>) -> Self {
        Self {
            deps: Some(deps.into_iter().collect()),
        }
    }

    pub fn initial(&self) -> OpFamilyState<O, T> {
        OpFamilyState::default()
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

#[derive(Clone, PartialEq, Eq, Hash, derive_more::Deref, derive_more::DerefMut)]
pub struct OpFamilyState<A: Id, T: Id>(BTreeMap<(A, T), OpFamilyPhase<A>>);

impl<A: Id, T: Id> Default for OpFamilyState<A, T> {
    fn default() -> Self {
        Self(BTreeMap::default())
    }
}

impl<A: Id, T: Id> OpFamilyState<A, T> {
    /// Get all items where the first element of the key is the specified input.
    /// Works for getting all ops of a given action.
    /// Assumes that the default value is also the minimum!
    /// TODO: use Min instead of Default
    pub fn iter_from(&self, key: A) -> impl Iterator<Item = (&(A, T), &OpFamilyPhase<A>)> {
        self.range((key, T::default())..)
            .take_while(move |(k, _)| k.0 == key)
    }

    pub fn find_awaiting(&self, key: A) -> impl Iterator<Item = A> + '_ {
        self.iter_from(key)
            .filter_map(move |(_, v)| v.try_unwrap_awaiting().ok().map(|(_, dep)| dep))
    }

    /// Returns true for every Integrated (valid) op,
    /// and false for every Rejected op
    pub fn find_integrated(&self, key: A) -> impl Iterator<Item = Outcome> + '_ {
        self.iter_from(key).filter_map(|(_, v)| match v {
            OpFamilyPhase::Op(OpPhase::Integrated(o)) => Some(*o),
            _ => None,
        })
    }
}

impl<A: Id, T: Id> Debug for OpFamilyState<A, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut l = f.debug_list();
        for ((a, t), phase) in self.iter() {
            l.entry(&format_args!("{a}.{t}: {phase:?}"));
        }
        l.finish()
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
            OpFamilyPhase::Op(p) if p.is_definitely_valid()
        )
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
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::From, Exhaustive, Serialize, Deserialize,
)]
pub enum OpFamilyAction<O: Id> {
    #[from]
    Op(OpAction),
    /// Await these ops
    Await(VT, O),
}

/// Given a btreemap of ops to their dependencies, detect if there are any loops
fn detect_loop<O: Id, T: Id>(state: &OpFamilyState<O, T>, id: O) -> bool {
    let mut visited = HashSet::new();
    let mut next = vec![id];
    while let Some(id) = next.pop() {
        for dep in state.find_awaiting(id) {
            if !visited.insert(dep) {
                return true;
            }
            next.push(dep);
        }
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
pub struct OpFamilyStatePretty<A: Id, T: Id>(pub OpFamilyState<A, T>);

impl<A: Id, T: Id> Debug for OpFamilyStatePretty<A, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ((id, t), state) in self.0.iter() {
            writeln!(f, "{id}.{t} = {state}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use polestar::{
        diagram::exhaustive::write_dot_state_diagram_mapped,
        id::{IdU8, IdUnit},
        machine::checked::Predicate,
        traversal::traverse_checked,
    };

    #[test]
    #[ignore = "nonterminating"]
    fn op_family_properties() {
        tracing_subscriber::fmt::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();

        use Predicate as P;

        type A = IdU8<3>;
        type T = IdU8<1>;

        let awaiting = |(a, t): (A, T), b: A| {
            P::atom(
                format!("{a}.{t} awaits {b}"),
                move |s: &OpFamilyState<A, T>| {
                    s.get(&(a, t))
                        .map(|p| matches!(p, OpFamilyPhase::Awaiting(_, d) if *d == b))
                        .unwrap_or(false)
                },
            )
        };

        // let awaiting = |a: A, b: A| {
        //     P::atom(format!("{a} awaits {b}"), move |s: &OpFamilyState<A, T>| {
        //         s.find_awaiting(a).any(|dep| dep == b)
        //     })
        // };

        let op_integrated = |(a, t)| {
            P::atom(
                format!("{a}.{t} integrated"),
                move |s: &OpFamilyState<A, T>| {
                    s.get(&(a, t))
                        .map(|p| {
                            matches!(p, OpFamilyPhase::Op(OpPhase::Integrated(Outcome::Accepted)))
                        })
                        .unwrap_or(false)
                },
            )
        };

        let action_integrated = |a| {
            T::iter_exhaustive(None)
                .map(|t| {
                    P::atom(
                        format!("{a}.{t} integrated"),
                        move |s: &OpFamilyState<A, T>| {
                            s.get(&(a, t))
                                .map(|p| {
                                    matches!(
                                        p,
                                        OpFamilyPhase::Op(OpPhase::Integrated(Outcome::Accepted))
                                    )
                                })
                                .unwrap_or(false)
                        },
                    )
                })
                .reduce(P::or)
                .unwrap()
        };

        let machine: OpFamilyMachine<A, T> = OpFamilyMachine::new();

        let predicates = <(A, T)>::iter_exhaustive(None)
            .flat_map(|(a, t)| {
                A::iter_exhaustive(None).flat_map(move |b| {
                    [
                        P::always(awaiting((a, t), b).implies(P::not(awaiting((b, t), a)))),
                        P::always(awaiting((a, t), b).implies(P::always(
                            op_integrated((a, t)).implies(action_integrated(b)),
                        ))),
                    ]
                })
            })
            .collect_vec();

        dbg!(&predicates);

        let checker = machine.clone().checked().with_predicates(predicates);

        let initial = checker.initial(machine.initial());

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
        // tracing_subscriber::fmt::fmt()
        //     .with_max_level(tracing::Level::DEBUG)
        //     .init();

        use polestar::diagram::exhaustive::DiagramConfig;

        type A = IdU8<2>;
        type T = IdU8<1>;

        // Create an instance of OpMachine with 1 dependency
        // let machine: OpFamilyMachine<A, T> = OpFamilyMachine::new_bounded(items);
        let machine: OpFamilyMachine<A, T> = OpFamilyMachine::new();

        let initial = machine.initial();

        write_dot_state_diagram_mapped(
            "op-family.dot",
            machine,
            initial,
            &DiagramConfig {
                trace_errors: true,
                ignore_loopbacks: true,
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
        type T = IdU8<2>;
        let o = O::all_values();
        let t = T::all_values();

        let v = VT::Sys;

        let state = OpFamilyState(
            [
                ((o[1], t[0]), OpFamilyPhase::Awaiting(v, o[2])),
                ((o[0], t[1]), OpFamilyPhase::Awaiting(v, o[1])),
                ((o[2], t[0]), OpFamilyPhase::Op(OpPhase::Stored)),
            ]
            .into_iter()
            .collect(),
        );
        assert!(!detect_loop(&state, o[0]));

        let state = OpFamilyState(
            [
                ((o[0], t[1]), OpFamilyPhase::Awaiting(v, o[1])),
                ((o[1], t[0]), OpFamilyPhase::Awaiting(v, o[2])),
                ((o[2], t[1]), OpFamilyPhase::Awaiting(v, o[0])),
            ]
            .into_iter()
            .collect(),
        );
        assert!(detect_loop(&state, o[0]));
    }
}
