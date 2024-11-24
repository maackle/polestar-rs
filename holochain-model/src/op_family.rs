use std::{
    collections::{BTreeMap, HashSet},
    fmt::Debug,
};

use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use polestar::{id::Id, prelude::*};

use crate::op_single::{OpEvent, OpPhase, OpSingleMachine, ValidationType as VT};

/// Machine that tracks the state of an op and all its dependencies
#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::Constructor)]
pub struct OpFamilyMachine<OpId: Id> {
    pub focus: OpId,
}

// impl<OpId: IdT> OpFamilyMachine<OpId> {
//     /// Create a new OpMachine with the given dependencies
//     pub fn new(id: OpId, deps: impl IntoIterator<Item = OpFamilyMachine<OpId>>) -> Self {
//         Self {
//             id,
//             deps: deps.into_iter().map(|d| (d.id, d)).collect(),
//         }
//     }
// }

#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::Deref, derive_more::DerefMut)]
pub struct OpFamilyState<OpId: Id>(BTreeMap<OpId, OpFamilyPhase<OpId>>);

impl<OpId: Id> OpFamilyState<OpId> {
    pub fn new(ids: impl IntoIterator<Item = OpId>) -> Self {
        Self(
            ids.into_iter()
                .map(|id| (id, OpFamilyPhase::default()))
                .collect(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display, derive_more::TryUnwrap)]
pub enum OpFamilyPhase<OpId: Id> {
    Op(OpPhase),
    #[display("Awaiting({}, {})", _0, _1)]
    Awaiting(VT, OpId),
}

impl<OpId: Id> Default for OpFamilyPhase<OpId> {
    fn default() -> Self {
        Self::Op(Default::default())
    }
}

impl<OpId: Id> OpFamilyPhase<OpId> {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, /* derive_more::Display, */ Exhaustive)]
pub enum OpFamilyEvent<OpId: Id> {
    Op(OpEvent),
    /// Await these ops
    Await(VT, OpId),
}

impl<OpId: Id> Machine for OpFamilyMachine<OpId> {
    type State = OpFamilyState<OpId>;
    type Action = (OpId, OpFamilyEvent<OpId>);
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(
        &self,
        mut states: Self::State,
        (target, action): Self::Action,
    ) -> MachineResult<Self> {
        use OpFamilyEvent as E;
        use OpFamilyPhase as S;
        use OpPhase::*;

        if let E::Await(_, dep) = action {
            if dep == target {
                bail!("An op can't depend on itself")
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
                (VT::Sys, Pending) => S::Awaiting(VT::Sys, dep_id),
                (VT::App, Validated(VT::Sys)) => S::Awaiting(VT::App, dep_id),
                _ => bail!("invalid transition to Awaiting: {state:?} -> {action:?}"),
            },

            // Transitions out of the Awaiting state
            (S::Awaiting(vt, dep_id), E::Op(a)) => match (vt, a) {
                (VT::Sys, OpEvent::Validate(VT::Sys)) | (VT::App, OpEvent::Validate(VT::App)) => {
                    let dep = states.get(&dep_id).ok_or(anyhow!("no dep {:?}", dep_id))?;
                    if dep.is_definitely_valid() {
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

#[derive(Clone, PartialEq, Eq, Hash, derive_more::From)]
pub struct OpFamilyStatePretty<I: Id>(OpFamilyState<I>);

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
    use polestar::{diagram::exhaustive::write_dot_state_diagram_mapped, id::IdU8};

    #[test]
    fn test_loop() {
        type O = IdU8<3>;

        let v = VT::Sys;

        let state: BTreeMap<_, _> = [
            (O::new(0), OpFamilyPhase::Awaiting(v, O::new(1))),
            (O::new(1), OpFamilyPhase::Awaiting(v, O::new(2))),
            (O::new(2), OpFamilyPhase::Op(OpPhase::Pending)),
        ]
        .into_iter()
        .collect();
        assert!(!detect_loop(&state, O::new(0)));

        let state: BTreeMap<_, _> = [
            (O::new(0), OpFamilyPhase::Awaiting(v, O::new(1))),
            (O::new(1), OpFamilyPhase::Awaiting(v, O::new(2))),
            (O::new(2), OpFamilyPhase::Awaiting(v, O::new(0))),
        ]
        .into_iter()
        .collect();
        assert!(detect_loop(&state, O::new(0)));
    }

    #[test]
    #[ignore = "diagram"]
    fn test_diagram() {
        use polestar::diagram::exhaustive::DiagramConfig;

        type OpId = IdU8<2>;

        // Create an instance of OpMachine with 1 dependency
        let machine: OpFamilyMachine<OpId> = OpFamilyMachine::new(OpId::new(0));

        write_dot_state_diagram_mapped(
            "op-family.dot",
            machine,
            OpFamilyState::new([OpId::new(0), OpId::new(1)]),
            &DiagramConfig {
                max_actions: Some(5),
                ..Default::default()
            },
            |state| OpFamilyStatePretty(state),
        );
    }
}
