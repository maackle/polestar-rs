use std::{collections::BTreeMap, fmt::Debug};

use anyhow::bail;
use exhaustive::Exhaustive;
use polestar::{id::IdT, prelude::*};

use crate::op_single::{OpEvent, OpPhase, OpSingleMachine, ValidationType as VT};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OpMachine<OpId: IdT> {
    id: OpId,
    deps: BTreeMap<OpId, OpMachine<OpId>>,
}

impl<OpId: IdT> OpMachine<OpId> {
    /// Create a new OpMachine with the given dependencies
    pub fn new(id: OpId, deps: impl IntoIterator<Item = OpMachine<OpId>>) -> Self {
        Self {
            id,
            deps: deps.into_iter().map(|d| (d.id, d)).collect(),
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct OpState<OpId: IdT> {
    phase: OpFamilyPhase<OpId>,
    deps: BTreeMap<OpId, OpFamilyPhase<OpId>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpFamilyPhase<OpId: IdT> {
    Op(OpPhase),
    Awaiting(VT, OpId),
}

impl<OpId: IdT> Default for OpFamilyPhase<OpId> {
    fn default() -> Self {
        Self::Op(Default::default())
    }
}

impl<OpId: IdT> OpFamilyPhase<OpId> {
    pub fn is_definitely_valid(&self) -> bool {
        matches!(
            self,
            OpFamilyPhase::Op(OpPhase::Validated(VT::App)) | OpFamilyPhase::Op(OpPhase::Integrated)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, /* derive_more::Display, */ Exhaustive)]
pub enum OpFamilyEvent<OpId: IdT> {
    Op(OpEvent),
    /// Await these ops
    Await(VT, OpId),
}

impl<OpId: IdT> Machine for OpMachine<OpId> {
    type State = OpFamilyPhase<OpId>;
    type Action = OpFamilyEvent<OpId>;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(&self, state: Self::State, action: Self::Action) -> MachineResult<Self> {
        use OpFamilyEvent as E;
        use OpFamilyPhase as S;

        if let E::Await(_, dep) = action {
            if dep == self.id {
                bail!("can't depend on self")
            }
        }

        let next = match (state, action) {
            (S::Op(s), E::Op(e)) => S::Op(OpSingleMachine.transition_(s, e)?),

            (S::Op(s), E::Await(vt, dep)) => match (vt, s) {
                (VT::Sys, OpPhase::Pending) => S::Awaiting(VT::Sys, dep),
                (VT::App, OpPhase::Validated(VT::Sys)) => S::Awaiting(VT::App, dep),
                _ => bail!("invalid transition: {state:?} -> {action:?}"),
            },

            (S::Awaiting(vt, _), E::Op(a)) => match (vt, a) {
                (VT::Sys, OpEvent::Validate(VT::Sys)) => S::Op(OpPhase::Validated(VT::Sys)),
                (VT::App, OpEvent::Validate(VT::App)) => S::Op(OpPhase::Validated(VT::App)),
                _ => bail!("invalid transition: {state:?} -> {action:?}"),
            },

            (state, action) => bail!("invalid transition: {state:?} -> {action:?}"),
        };
        Ok((next, ()))
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        matches!(
            state,
            OpFamilyPhase::Op(OpPhase::Integrated | OpPhase::Rejected)
        )
    }
}

// impl<OpId: IdT> AutoMapped for OpMachine<OpId> {
//     type Key = OpId;

//     fn key(&self) -> &Self::Key {
//         &self.id
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use polestar::{diagram::exhaustive::write_dot_state_diagram, id::IdU8};

    #[test]
    #[ignore = "diagram"]
    fn test_diagram() {
        use polestar::diagram::exhaustive::DiagramConfig;

        type OpId = IdU8<2>;

        // Create an instance of OpMachine with 1 dependency
        let machine: OpMachine<OpId> =
            OpMachine::new(OpId::new(0), [OpMachine::new(OpId::new(1), [])]);

        write_dot_state_diagram(
            "op-family.dot",
            machine,
            OpFamilyPhase::default(),
            &DiagramConfig {
                max_actions: Some(5),
                ..Default::default()
            },
        );
    }
}
