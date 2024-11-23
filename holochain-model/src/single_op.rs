use std::{collections::BTreeSet, fmt::Debug, marker::PhantomData};

use anyhow::bail;
use exhaustive::Exhaustive;
use polestar::{id::IdT, prelude::*};
use proptest_derive::Arbitrary;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OpMachine<NodeId: IdT, OpId: IdT> {
    deps: BTreeSet<OpId>,
    _phantom: PhantomData<NodeId>,
}

impl<NodeId: IdT, OpId: IdT> OpMachine<NodeId, OpId> {
    /// Create a new OpMachine with the given dependencies
    pub fn new(deps: impl IntoIterator<Item = OpId>) -> Self {
        Self {
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
    /// The last validation attempt could not complete due to missing dependencies
    Awaiting(ValidationType, BTreeSet<OpId>),
    /// The op has been validated
    Validated(ValidationType),
    /// The op has been rejected
    Rejected,
    /// The op has been integrated
    Integrated,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Arbitrary, derive_more::Display, Exhaustive)]
pub enum ValidationType {
    Sys,
    App,
}

use ValidationType as VT;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Arbitrary, /* derive_more::Display, */ Exhaustive)]
pub enum OpEvent<NodeId: IdT, OpId: IdT> {
    /// Author the op
    Author,
    /// Validate the op (as valid)
    Validate(ValidationType),
    /// Await these ops
    Await(BTreeSet<OpId>),
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

        let next = match (state, t) {
            // Receive the op
            (S::None, E::Author) => S::Pending,

            // Duplicate authorship is an error
            (_, E::Author) => bail!("duplicate authorship"),

            (S::Pending | S::Validated(V::Sys), E::Reject) => S::Rejected,
            (S::Pending, E::Await(deps)) => S::Awaiting(VT::Sys, deps),
            (S::Validated(V::Sys), E::Await(deps)) => S::Awaiting(VT::App, deps),

            (S::Pending, E::Validate(V::Sys)) => S::Validated(V::Sys),

            (S::Validated(V::Sys), E::Validate(V::App)) => S::Validated(V::App),

            (S::Validated(V::App), E::Integrate) => S::Integrated,

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

        // Create an instance of OpMachine with empty dependencies
        let machine = OpMachine::<NodeId, OpId>::new([OpId::modulo(0)]);

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
