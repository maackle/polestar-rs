use std::{collections::HashSet, fmt::Debug};

use anyhow::bail;
use polestar::{id::Id, prelude::*};

use crate::op_family::{OpFamilyAction, OpFamilyMachine, OpFamilyState};

/// Machine that tracks the state of an op and all its dependencies
#[derive(Clone, Debug)]
pub struct OpFamilyKnownDepsMachine<O: Id> {
    pub machine: OpFamilyMachine<O>,
    pub allowed_pairs: HashSet<(O, O)>,
}

impl<O: Id> Machine for OpFamilyKnownDepsMachine<O> {
    type State = OpFamilyState<O>;
    type Action = (O, OpFamilyAction<O>);
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(
        &self,
        state: Self::State,
        (target, action): Self::Action,
    ) -> TransitionResult<Self> {
        use OpFamilyAction as E;

        if let E::Await(_, dep_id) = action {
            if !self.allowed_pairs.contains(&(target, dep_id)) {
                bail!("dependency not specified in machine: {target:?} -> {dep_id:?}");
            }
        }

        self.machine.transition(state, (target, action))
    }

    fn is_terminal(&self, s: &Self::State) -> bool {
        self.machine.is_terminal(s)
    }
}

impl<O: Id> OpFamilyKnownDepsMachine<O> {
    pub fn new(focus: O, allowed_pairs: impl IntoIterator<Item = (O, O)>) -> Self {
        let allowed_pairs: HashSet<(O, O)> = allowed_pairs.into_iter().collect();
        let deps = allowed_pairs.iter().flat_map(|(x, y)| [x, y]).copied();
        let machine = OpFamilyMachine::new(focus, deps);
        Self {
            machine,
            allowed_pairs,
        }
    }

    pub fn initial(&self) -> OpFamilyState<O> {
        OpFamilyState::new(self.machine.deps.clone())
    }
}

#[cfg(test)]
mod tests {
    use polestar::{diagram::exhaustive::write_dot_state_diagram_mapped, id::IdU8};

    use crate::op_family::OpFamilyStatePretty;

    use super::*;

    #[test]
    #[ignore = "diagram"]
    fn test_op_family_known_deps_diagram() {
        use polestar::diagram::exhaustive::DiagramConfig;

        type O = IdU8<3>;
        let o = O::all_values();

        let pairs = [(o[0], o[1]), (o[1], o[2])];

        // Create an instance of OpMachine with 1 dependency
        let machine: OpFamilyKnownDepsMachine<O> = OpFamilyKnownDepsMachine::new(o[0], pairs);

        let initial = machine.initial();

        write_dot_state_diagram_mapped(
            "op-family-known-deps.dot",
            machine,
            initial,
            &DiagramConfig {
                max_actions: Some(5),
                ..Default::default()
            },
            |state| OpFamilyStatePretty(state),
            |action| action,
        );
    }
}
