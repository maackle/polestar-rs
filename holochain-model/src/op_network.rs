use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
};

use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use polestar::{ext::MapExt, id::Id, Machine, TransitionResult};
use serde::{Deserialize, Serialize};

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
pub struct OpNetworkMachine<N: Id, O: Id, T: Id> {
    pub inner: OpFamilyMachine<O, T>,
    pub nodes: Option<BTreeSet<N>>,
}

impl<N: Id, O: Id, T: Id> Machine for OpNetworkMachine<N, O, T> {
    type State = OpNetworkState<N, O, T>;
    type Action = (N, OpNetworkAction<N, O, T>);
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(
        &self,
        mut state: Self::State,
        (node, action): Self::Action,
    ) -> TransitionResult<Self> {
        // If nodes aren't bounded, add a new node when seen
        if self.nodes.is_none() && !state.nodes.contains_key(&node) {
            state.nodes.insert(node, self.inner.initial());
        }

        let fx = state
            .nodes
            .owned_update(node, |nodes, node_state| match action {
                OpNetworkAction::Local { op, action } => {
                    self.inner.transition(node_state, (op, action))
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
                        .transition(node_state, (op, OpAction::Store(false).into()))
                }
            })?;

        Ok((state, fx))
    }

    fn is_terminal(&self, s: &Self::State) -> bool {
        s.nodes
            .values()
            .all(|node_state| self.inner.is_terminal(node_state))
    }
}

impl<N: Id, O: Id, T: Id> OpNetworkMachine<N, O, T> {
    pub fn new() -> Self {
        Self {
            inner: OpFamilyMachine::new(),
            nodes: None,
        }
    }

    pub fn new_bounded(
        nodes: impl IntoIterator<Item = N>,
        ops: impl IntoIterator<Item = (O, T)>,
    ) -> Self {
        Self {
            inner: OpFamilyMachine::new_bounded(ops),
            nodes: Some(nodes.into_iter().collect()),
        }
    }

    pub fn initial(&self) -> OpNetworkState<N, O, T> {
        let nodes = self
            .nodes
            .as_ref()
            .map(|ns| {
                ns.iter()
                    .copied()
                    .map(|n| (n, self.inner.initial()))
                    .collect()
            })
            .unwrap_or_default();
        OpNetworkState { nodes }
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
pub struct OpNetworkState<N: Id, O: Id, T: Id> {
    pub nodes: BTreeMap<N, OpFamilyState<O, T>>,
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

pub type OpNetworkMachineAction<N, O, T> = (N, OpNetworkAction<N, O, T>);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Exhaustive, Serialize, Deserialize)]
pub enum OpNetworkAction<N: Id, O: Id, T: Id> {
    Local {
        op: (O, T),
        action: OpFamilyAction<O>,
    },
    Receive {
        op: (O, T),
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
pub struct OpNetworkStatePretty<N: Id, O: Id, T: Id>(pub OpNetworkState<N, O, T>);

impl<N: Id, O: Id, T: Id> Debug for OpNetworkStatePretty<N, O, T> {
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
pub struct OpNetworkEdgePretty<N: Id, O: Id, T: Id>(N, pub OpNetworkAction<N, O, T>);

impl<N: Id, O: Id, T: Id> Debug for OpNetworkEdgePretty<N, O, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(node, action) = self;
        match action {
            OpNetworkAction::Local { op: (a, t), action } => {
                write!(f, "[{node}, {a}.{t}] {action:?}",)?
            }
            OpNetworkAction::Receive {
                op: (a, t),
                from,
                valid,
            } => write!(f, "{node} ↢ {from}: Recv({a}.{t}, {valid})",)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use polestar::{
        diagram::exhaustive::write_dot_state_diagram_mapped, id::IdU8,
        machine::checked::Predicate as P,
    };

    use super::*;

    #[test]
    #[ignore = "diagram"]
    fn test_op_network_diagram() {
        use polestar::diagram::exhaustive::DiagramConfig;

        type N = IdU8<2>;
        type O = IdU8<1>;
        type T = IdU8<1>;

        let n = N::all_values();
        let o = O::all_values();
        let t = T::all_values();
        let ops = <(O, T)>::iter_exhaustive(None).collect_vec();

        // Create an instance of OpMachine with 1 dependency
        let machine: OpNetworkMachine<N, O, T> = OpNetworkMachine::new_bounded(n, ops);
        let initial = machine.initial();

        // let all_integrated = {
        //     let m = machine.clone();
        //     P::atom("integrated", move |s| m.is_terminal(s))
        // };
        // let predicates = [P::not(P::eventually(all_integrated))];
        // // let predicates = [];
        // let machine = machine.checked().with_predicates(predicates);
        // let initial = machine.initial(initial);

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
