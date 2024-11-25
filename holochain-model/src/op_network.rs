use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
};

use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use polestar::{ext::MapExt, id::Id, Machine, TransitionResult};

use crate::{
    op_family::{OpFamilyAction, OpFamilyMachine, OpFamilyPhase, OpFamilyState},
    op_single::{OpAction, OpPhase, ValidationType},
};

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OpNetworkMachine<N: Id, O: Id> {
    pub inner: OpFamilyMachine<O>,
    pub nodes: BTreeSet<N>,
}

impl<N: Id, O: Id> Machine for OpNetworkMachine<N, O> {
    type State = OpNetworkState<N, O>;
    type Action = (N, OpNetworkAction<N, O>);

    fn transition(
        &self,
        mut state: Self::State,
        (node, action): Self::Action,
    ) -> TransitionResult<Self> {
        state
            .nodes
            .owned_update(node, |nodes, node_state| match action {
                OpNetworkAction::Family { target, action } => {
                    self.inner.transition_(node_state, (target, action))
                }
                OpNetworkAction::Receive { op, from, valid } => {
                    let from_phase = nodes
                        .get(&from)
                        .ok_or(anyhow!("no node"))?
                        .get(&op)
                        .ok_or(anyhow!("no op"))?;

                    if !matches!(from_phase, OpFamilyPhase::Op(OpPhase::Integrated)) {
                        bail!("can't receive op if target has not integrated")
                    }

                    if !valid {
                        bail!("invalid op not currently handled")
                    }

                    self.inner
                        .transition_(node_state, (op, OpAction::Store.into()))
                }
            })?;

        Ok((state, ()))
    }

    fn is_terminal(&self, _: &Self::State) -> bool {
        false
    }
}

impl<N: Id, O: Id> OpNetworkMachine<N, O> {
    pub fn new(
        nodes: impl IntoIterator<Item = N>,
        root_op: O,
        ops: impl IntoIterator<Item = O>,
    ) -> Self {
        Self {
            inner: OpFamilyMachine::new(root_op, ops),
            nodes: nodes.into_iter().collect(),
        }
    }

    pub fn initial(&self) -> OpNetworkState<N, O> {
        OpNetworkState {
            nodes: self
                .nodes
                .iter()
                .copied()
                .map(|n| (n, self.inner.initial(self.inner.deps.clone())))
                .collect(),
        }
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::From)]
pub struct OpNetworkState<N: Id, O: Id> {
    pub nodes: BTreeMap<N, OpFamilyState<O>>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Exhaustive)]
pub enum OpNetworkAction<N: Id, O: Id> {
    Family {
        target: O,
        action: OpFamilyAction<O>,
    },
    Receive {
        op: O,
        from: N,
        valid: bool,
    },
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
pub struct OpNetworkStatePretty<N: Id, O: Id>(pub OpNetworkState<N, O>);

impl<N: Id, I: Id> Debug for OpNetworkStatePretty<N, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (n, node) in self.0.nodes.iter() {
            write!(f, "n{n} [ ")?;
            for (_, state) in node.iter() {
                write!(f, "{state}")?;
            }
            write!(f, " ]\n")?;
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Hash, derive_more::From)]
pub struct OpNetworkEdgePretty<N: Id, O: Id>(N, pub OpNetworkAction<N, O>);

impl<N: Id, I: Id> Debug for OpNetworkEdgePretty<N, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(node, action) = self;
        match action {
            OpNetworkAction::Family { target, action } => {
                write!(f, "[{node}, {target}] {action:?}",)?
            }
            OpNetworkAction::Receive { op, from, valid } => {
                write!(f, "{node} ↢ {from}: Recv({op}, {valid})",)?
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use polestar::{diagram::exhaustive::write_dot_state_diagram_mapped, id::IdU8};

    use super::*;

    #[test]
    #[ignore = "diagram"]
    fn test_op_network_diagram() {
        use polestar::diagram::exhaustive::DiagramConfig;

        type N = IdU8<2>;
        type O = IdU8<1>;

        let n = N::all_values();
        let o = O::all_values();

        // Create an instance of OpMachine with 1 dependency
        let machine: OpNetworkMachine<N, O> = OpNetworkMachine::new(n, o[0], o);

        let initial = machine.initial();

        write_dot_state_diagram_mapped(
            "op-network.dot",
            machine,
            initial,
            &DiagramConfig {
                ..Default::default()
            },
            |state| OpNetworkStatePretty(state),
            |(target, action)| OpNetworkEdgePretty(target, action),
        );
    }
}
