use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt::{Debug, Display, Pointer},
};

use anyhow::{anyhow, bail};
use derive_more::derive::Deref;
use exhaustive::Exhaustive;
use itertools::Itertools;
use polestar::{
    id::{Id, UpTo},
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
    pub deps: Option<BTreeSet<OpId<O, T>>>,
}

impl<O: Id, T: Id> Machine for OpFamilyMachine<O, T> {
    type State = OpFamilyState<O, T>;
    type Action = (OpId<O, T>, OpFamilyAction<O>);
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

        if !self.is_op_handled(&target) {
            bail!("{target:?} not covered");
        }

        // Add a new state when seen if we didn't already bail.
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
                bail!("The focus op (first dep) can't be depended on")
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
                            "attempted to validate op {target} but still awaiting dep {:?}",
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

    fn is_terminal(&self, _state: &Self::State) -> bool {
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

    pub fn new_bounded(deps: impl IntoIterator<Item = OpId<O, T>>) -> Self {
        Self {
            deps: Some(deps.into_iter().collect()),
        }
    }

    pub fn initial(&self) -> OpFamilyState<O, T> {
        OpFamilyState::default()
    }

    /// If the machine specifies a set of deps, then the target op must be in that set.
    /// Otherwise, any op is fair game.
    pub fn is_op_handled(&self, target: &OpId<O, T>) -> bool {
        self.deps
            .as_ref()
            .map(|ds| ds.contains(target))
            .unwrap_or(true)
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
pub struct OpFamilyState<A: Id, T: Id>(BTreeMap<OpId<A, T>, OpFamilyPhase<A>>);

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
    pub fn iter_from(&self, key: A) -> impl Iterator<Item = (&OpId<A, T>, &OpFamilyPhase<A>)> {
        self.range(OpId(key, T::default())..)
            .take_while(move |(k, _)| k.0 == key)
    }

    /// For every op with the given dep key whose status is Awaiting,
    /// return the awaited deps
    pub fn all_awaiting(&self, key: A) -> impl Iterator<Item = A> + '_ {
        self.iter_from(key)
            .filter_map(move |(_, v)| v.try_unwrap_awaiting().ok().map(|(_, dep)| dep))
    }

    /// Returns true for every Integrated (valid) op,
    /// and false for every Rejected op
    pub fn all_integrated(&self, key: A) -> impl Iterator<Item = Outcome> + '_ {
        self.iter_from(key).filter_map(|(_, v)| match v {
            OpFamilyPhase::Op(OpPhase::Integrated(o)) => Some(*o),
            _ => None,
        })
    }
}

impl<A: Id, T: Id> Debug for OpFamilyState<A, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut l = f.debug_list();
        for (id, phase) in self.iter() {
            l.entry(&format_args!("{id}: {phase:?}"));
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

#[derive(
    Clone,
    Copy,
    // Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Exhaustive,
    Serialize,
    Deserialize,
    derive_more::From,
)]
pub struct OpId<O, T>(pub O, pub T);

impl<O: Id, T: Id> std::fmt::Display for OpId<O, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

impl<O: Id, T: Id> Debug for OpId<O, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OpId({}.{})", self.0, self.1)
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
        for dep in state.all_awaiting(id) {
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

#[derive(Clone, PartialEq, Eq, Hash, Debug, derive_more::From)]
pub struct OpFamilyStatePretty<A: Id, T: Id>(pub OpFamilyState<A, T>);

impl<A: Id, T: Id> Display for OpFamilyStatePretty<A, T> {
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
        diagram::exhaustive::write_dot_state_diagram_mapped,
        id::{IdUnit, UpTo},
        logic::{conjoin, PropRegistry, Propositions, Transition},
        model_checker::{model_checker_report, ModelChecker},
        traversal::TraversalConfig,
    };

    #[test]
    fn op_family_properties() {
        tracing_subscriber::fmt::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        type A = UpTo<2>;
        type T = UpTo<1>;

        #[derive(Clone, PartialEq, Eq)]
        enum Prop {
            OpAwaiting(OpId<A, T>, A),
            ActionAwaiting(A, A),
            OpIntegrated(OpId<A, T>),
            ActionIntegrated(A),
        }

        impl std::fmt::Display for Prop {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Prop::OpAwaiting(o, d) => write!(f, "{o}_awaits_{d}"),
                    Prop::ActionAwaiting(o, a) => write!(f, "{o}_awaits_{a}"),
                    Prop::OpIntegrated(o) => write!(f, "integrated_{o}"),
                    Prop::ActionIntegrated(a) => write!(f, "integrated_{a}"),
                }
            }
        }

        impl Propositions<Prop> for Transition<OpFamilyMachine<A, T>> {
            fn eval(&self, prop: &Prop) -> bool {
                let Transition(state, _, _) = self;
                match prop {
                    Prop::OpAwaiting(o, b) => state
                        .get(&o)
                        .map(|p| matches!(p, OpFamilyPhase::Awaiting(_, d) if d == b))
                        .unwrap_or(false),

                    Prop::ActionAwaiting(a, b) => state.all_awaiting(*a).any(|d| d == *b),

                    Prop::OpIntegrated(o) => state
                        .get(&o)
                        .map(|p| {
                            matches!(p, OpFamilyPhase::Op(OpPhase::Integrated(Outcome::Accepted)))
                        })
                        .unwrap_or(false),

                    Prop::ActionIntegrated(a) => T::iter_exhaustive(None)
                        .map(|t| {
                            let o = OpId(*a, t);
                            state
                                .get(&o)
                                .map(|p| {
                                    matches!(
                                        p,
                                        OpFamilyPhase::Op(OpPhase::Integrated(Outcome::Accepted))
                                    )
                                })
                                .unwrap_or(false)
                        })
                        .reduce(|a, b| a || b)
                        .unwrap(),
                }
            }
        }

        let pairs = <OpId<A, T>>::iter_exhaustive(None)
            .flat_map(|o| {
                A::iter_exhaustive(None).filter_map(move |b| (o.0 != b).then_some((o, b)))
            })
            .collect_vec();

        let mut propmap = PropRegistry::empty();

        let machine: OpFamilyMachine<A, T> = OpFamilyMachine::new();

        let ltl = conjoin(pairs.into_iter().flat_map(|(o, b)| {
            let ow = propmap.add(Prop::OpAwaiting(o, b)).unwrap();
            let aw = propmap.add(Prop::ActionAwaiting(b, o.0)).unwrap();
            let oi = propmap.add(Prop::OpIntegrated(o)).unwrap();
            let ai = propmap.add(Prop::ActionIntegrated(b)).unwrap();
            vec![
                format!("G ({ow} -> !{aw} )"),
                format!("G ({ow} -> (G {oi} -> {ai} ) )"),
            ]
        }));

        let initial = machine.initial();
        let checker = ModelChecker::new(machine.clone(), propmap, &ltl).unwrap();

        model_checker_report(checker.check(initial));
    }

    #[test]
    #[ignore = "diagram"]
    fn test_op_family_diagram() {
        // tracing_subscriber::fmt::fmt()
        //     .with_max_level(tracing::Level::DEBUG)
        //     .init();

        use polestar::diagram::exhaustive::DiagramConfig;

        type A = UpTo<2>;
        type T = UpTo<1>;

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
            |state| Some(OpFamilyStatePretty(state)),
            |action| Some(format!("{action:?}")),
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

        type O = UpTo<4>;
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
        type O = UpTo<3>;
        type T = UpTo<2>;
        let o = O::all_values();
        let t = T::all_values();

        let v = VT::Sys;

        let state = OpFamilyState(
            [
                ((o[1], t[0]).into(), OpFamilyPhase::Awaiting(v, o[2])),
                ((o[0], t[1]).into(), OpFamilyPhase::Awaiting(v, o[1])),
                ((o[2], t[0]).into(), OpFamilyPhase::Op(OpPhase::Stored)),
            ]
            .into_iter()
            .collect(),
        );
        assert!(!detect_loop(&state, o[0]));

        let state = OpFamilyState(
            [
                ((o[0], t[1]).into(), OpFamilyPhase::Awaiting(v, o[1])),
                ((o[1], t[0]).into(), OpFamilyPhase::Awaiting(v, o[2])),
                ((o[2], t[1]).into(), OpFamilyPhase::Awaiting(v, o[0])),
            ]
            .into_iter()
            .collect(),
        );
        assert!(detect_loop(&state, o[0]));
    }
}
