use std::fmt::Debug;

use anyhow::bail;
use exhaustive::Exhaustive;
use polestar::prelude::*;
use serde::{Deserialize, Serialize};

pub struct OpSingleMachine;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, derive_more::Display)]
pub enum OpPhase {
    #[default]
    /// The op has not been seen by this node yet
    None,

    /// The op has been received, either first-hand (authorship) or second-hand (via publish or gossip)
    /// and validation has not yet been attempted
    Stored,

    /// The op has passed validation.
    #[display("Validated({})", _0)]
    Validated(ValidationType),

    /// The op has been rejected
    Rejected,

    /// The op has been integrated
    Integrated(Outcome),
}

impl OpPhase {
    pub fn is_definitely_valid(&self) -> bool {
        matches!(
            self,
            OpPhase::Validated(VT::App) | OpPhase::Integrated(Outcome::Accepted)
        )
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
    Exhaustive,
    Serialize,
    Deserialize,
)]
pub enum ValidationType {
    Sys,
    App,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
    Exhaustive,
    Serialize,
    Deserialize,
)]
pub enum Outcome {
    Accepted,
    Rejected,
}

impl Outcome {
    pub fn is_valid(&self) -> bool {
        matches!(self, Outcome::Accepted)
    }
}

use ValidationType as VT;

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
    Exhaustive,
    Serialize,
    Deserialize,
)]
pub enum OpAction {
    /// Store the op. If true, integrate immediately.
    Store(bool),
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
            // Author the op (it gets immediately integrated)
            (S::None, E::Store(true)) => S::Integrated(Outcome::Accepted),

            // Receive the op
            (S::None, E::Store(false)) => S::Stored,

            // Duplicate authorship should be an error,
            // but with the loose hookup to Holochain, we let it be idempotent.
            // (_, E::Store) => bail!("duplicate authorship"),
            (s, E::Store(_)) => s,

            // Here's some other idempotency additions I'm adding, but should be cleaned up.
            (s @ S::Validated(v1), E::Validate(v2)) if v1 == v2 => s,

            (S::Stored | S::Validated(V::Sys), E::Reject) => S::Rejected,
            (S::Stored, E::Validate(V::Sys)) => S::Validated(VT::Sys),
            (S::Validated(V::Sys), E::Validate(V::App)) => S::Validated(V::App),

            (S::Validated(V::App), E::Integrate) => S::Integrated(Outcome::Accepted),
            (S::Rejected, E::Integrate) => S::Integrated(Outcome::Rejected),

            // XXX: Allow idempotent integration, because Holochain does this.
            (S::Integrated(Outcome::Accepted), E::Integrate) => S::Integrated(Outcome::Accepted),

            (state, action) => bail!("invalid transition: {state:?} -> {action:?}"),
        };
        Ok((next, ()))
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        matches!(state, OpPhase::Integrated(_))
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
                ignore_loopbacks: true,
                ..Default::default()
            },
        );
    }
}
