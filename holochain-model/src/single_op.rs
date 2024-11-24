use std::{collections::BTreeSet, fmt::Debug, marker::PhantomData};

use anyhow::bail;
use exhaustive::Exhaustive;
use polestar::{id::IdT, prelude::*};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OpMachine<NodeId: IdT, OpId: IdT> {
    id: OpId,
    deps: BTreeSet<OpId>,
    _phantom: PhantomData<NodeId>,
}

impl<NodeId: IdT, OpId: IdT> OpMachine<NodeId, OpId> {
    /// Create a new OpMachine with the given dependencies
    pub fn new(id: OpId, deps: impl IntoIterator<Item = OpId>) -> Self {
        Self {
            id,
            deps: deps.into_iter().collect(),
            _phantom: PhantomData,
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub enum OpPhase<OpId: IdT> {
    #[default]
    /// The op has not been seen by this node yet
    None,
    /// The op has been received and validation has not been attempted
    Pending,
    /// The op has been validated.
    /// If the optional OpId is Some, validation is in limbo, awaiting the ops in the set.
    /// If it's None, then the validation of this type is complete.
    Validated(ValidationType, Option<OpId>),
    /// The op has been rejected
    Rejected,
    /// The op has been integrated
    Integrated,
}

impl<OpId: IdT> OpPhase<OpId> {
    pub fn is_definitely_valid(&self) -> bool {
        matches!(
            self,
            OpPhase::Validated(VT::App, None) | OpPhase::Integrated
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::Display, Exhaustive)]
pub enum ValidationType {
    Sys,
    App,
}

use ValidationType as VT;

#[derive(Clone, Debug, PartialEq, Eq, Hash, /* derive_more::Display, */ Exhaustive)]
pub enum OpEvent<NodeId: IdT, OpId: IdT> {
    /// Author the op
    Author,
    /// Validate the op (as valid)
    Validate(ValidationType),
    /// Await these ops
    Await(ValidationType, OpId),
    /// Reject the op (as invalid)
    Reject,
    /// Integrate the op
    Integrate,
    /// Send the op to another node
    Send(NodeId),
}

impl<NodeId: IdT, OpId: IdT> Machine for OpMachine<NodeId, OpId> {
    type State = OpPhase<OpId>;
    type Action = OpEvent<NodeId, OpId>;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(&self, state: Self::State, t: Self::Action) -> MachineResult<Self> {
        use OpEvent as E;
        use OpPhase as S;
        use ValidationType as V;

        if let E::Await(_, dep) = t {
            if dep == self.id {
                bail!("can't depend on self")
            }
        }

        let next = match (state, t) {
            // Receive the op
            (S::None, E::Author) => S::Pending,

            // Duplicate authorship is an error
            (_, E::Author) => bail!("duplicate authorship"),

            (S::Pending | S::Validated(V::Sys, _), E::Reject) => S::Rejected,
            (S::Pending, E::Await(V::Sys, dep)) => S::Validated(VT::Sys, Some(dep)),
            (S::Validated(V::Sys, Some(_)), E::Await(V::App, dep)) => {
                S::Validated(VT::App, Some(dep))
            }

            (S::Pending, E::Validate(V::Sys)) => S::Validated(VT::Sys, None),

            (S::Validated(V::Sys, Some(_)), E::Validate(V::Sys)) => S::Validated(V::Sys, None),
            (S::Validated(V::Sys, None), E::Validate(V::App)) => S::Validated(V::App, None),

            (S::Validated(V::App, Some(_)), E::Validate(V::App)) => S::Validated(V::App, None),
            (S::Validated(V::App, None), E::Integrate) => S::Integrated,

            (S::Integrated, E::Send(_)) => S::Integrated,

            (state, action) => bail!("invalid transition: {state:?} -> {action:?}"),
        };
        Ok((next, ()))
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        matches!(state, OpPhase::Integrated | OpPhase::Rejected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use polestar::{diagram::exhaustive::write_dot_state_diagram, id::Id};

    #[test]
    #[ignore = "diagram"]
    fn test_diagram() {
        use polestar::diagram::exhaustive::DiagramConfig;

        type OpId = Id<2>;
        type NodeId = Id<2>;

        // Create an instance of OpMachine with 1 dependency
        let machine = OpMachine::<NodeId, OpId>::new(OpId::new(0), [OpId::new(1)]);

        write_dot_state_diagram(
            "single-op.dot",
            machine,
            OpPhase::<OpId>::None,
            &DiagramConfig {
                max_actions: Some(5),
                ..Default::default()
            },
        );
    }
}
