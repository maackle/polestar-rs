use std::fmt::Debug;

use anyhow::bail;
use exhaustive::Exhaustive;
use polestar::prelude::*;

pub struct OpSingleMachine;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub enum OpPhase {
    #[default]
    /// The op has not been seen by this node yet
    None,
    /// The op has been received and validation has not been attempted
    Pending,
    /// The op has been validated.
    Validated(ValidationType),
    /// The op has been rejected
    Rejected,
    /// The op has been integrated
    Integrated,
}

impl OpPhase {
    pub fn is_definitely_valid(&self) -> bool {
        matches!(self, OpPhase::Validated(VT::App) | OpPhase::Integrated)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display, Exhaustive)]
pub enum ValidationType {
    Sys,
    App,
}

use ValidationType as VT;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, /* derive_more::Display, */ Exhaustive)]
pub enum OpEvent {
    /// Author the op
    Author,
    /// Validate the op (as valid)
    Validate(ValidationType),
    /// Reject the op (as invalid)
    Reject,
    /// Integrate the op
    Integrate,
}

impl Machine for OpSingleMachine {
    type State = OpPhase;
    type Action = OpEvent;
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
            (S::Pending, E::Validate(V::Sys)) => S::Validated(VT::Sys),
            (S::Validated(V::Sys), E::Validate(V::App)) => S::Validated(V::App),
            (S::Validated(V::App), E::Integrate) => S::Integrated,

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
    use polestar::diagram::exhaustive::write_dot_state_diagram;

    #[test]
    #[ignore = "diagram"]
    fn test_diagram() {
        use polestar::diagram::exhaustive::DiagramConfig;

        write_dot_state_diagram(
            "op-single.dot",
            OpSingleMachine,
            OpPhase::None,
            &DiagramConfig {
                max_actions: Some(5),
                ..Default::default()
            },
        );
    }
}
