use std::fmt::Debug;

use anyhow::bail;
use exhaustive::Exhaustive;
use polestar::prelude::*;

pub struct OpSingleMachine;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, derive_more::Display)]
pub enum OpPhase {
    #[default]
    /// The op has not been seen by this node yet
    None,
    /// The op has been received, either first-hand (authorship) or second-hand (via publish or gossip)
    /// and validation has not yet been attempted
    Stored,
    /// The op has been validated.
    #[display("Validated({})", _0)]
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
pub enum OpAction {
    /// Store the op
    Store,
    /// Validate the op (as valid)
    Validate(ValidationType),
    /// Reject the op (as invalid)
    Reject,
    /// Integrate the op
    Integrate,
}

impl Machine for OpSingleMachine {
    type State = OpPhase;
    type Action = OpAction;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(&self, state: Self::State, t: Self::Action) -> TransitionResult<Self> {
        use OpAction as E;
        use OpPhase as S;
        use ValidationType as V;

        let next = match (state, t) {
            // Receive the op
            (S::None, E::Store) => S::Stored,

            // Duplicate authorship is an error
            (_, E::Store) => bail!("duplicate authorship"),

            (S::Stored | S::Validated(V::Sys), E::Reject) => S::Rejected,
            (S::Stored, E::Validate(V::Sys)) => S::Validated(VT::Sys),
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
