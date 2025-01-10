use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Display},
};

use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use polestar::{ext::MapExt, id::Id, Machine, TransitionResult};
use serde::{Deserialize, Serialize};

use crate::{
    op_family::{OpFamilyAction, OpFamilyMachine, OpFamilyPhase, OpFamilyState, OpId},
    op_single::{OpAction, OpPhase, Outcome, ValidationType},
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
        let () = state.nodes.owned_upsert(
            node,
            |_| {
                // If nodes aren't bounded, add a new node when seen
                if self.nodes.is_none() {
                    Ok(self.inner.initial())
                } else {
                    Err(anyhow!("no node {node:?}"))
                }
            },
            |nodes, mut node_state| match action {
                OpNetworkAction::Local { op, action } => {
                    // self.inner.transition(node_state, (op, action))

                    use OpFamilyAction as E;
                    use OpFamilyPhase as S;
                    use OpPhase as P;
                    use ValidationType as VT;

                    let changed = node_state.owned_upsert(
                        op,
                        |_| {
                            if self.inner.is_op_handled(&op) {
                                Ok(OpFamilyPhase::default())
                            } else {
                                Err(anyhow!("no op {op:?}"))
                            }
                        },
                        |_, mut op_state| match (op_state, action) {
                            // If attempting to Validate while in an Awaiting state...
                            (S::Awaiting(vt, dep_id), E::Op(a)) => match (vt, a) {
                                (vt, OpAction::Validate(vt2)) if vt == vt2 => {
                                    // ... check if any nodes contain an non-rejected copy of the op,
                                    // which means they are able to serve it up via get request
                                    // NOTE: this does not actually model the get request! just the availablility
                                    // TODO: model the get request explicitly once kitsune can know who we got it from
                                    let any_not_rejected = nodes.values().any(|op_states| {
                                        op_states.iter_from(dep_id).any(|(_, op_state)| {
                                            // TODO: is this correct? Does a node serve up unintegrated ops?
                                            // or only integrated?
                                            !matches!(
                                                op_state,
                                                S::Op(P::Integrated(Outcome::Rejected))
                                            )
                                        })
                                    });

                                    if !any_not_rejected {
                                        bail!("can't validate op if all deps are rejected");
                                    }

                                    op_state = S::Op(P::Validated(vt));

                                    Ok((op_state, true))
                                }
                                _ => Ok((op_state, false)),
                            },
                            _ => Ok((op_state, false)),
                        },
                    )?;
                    if changed {
                        Ok((node_state, ()))
                    } else {
                        self.inner.transition(node_state, (op, action))
                    }
                }
                OpNetworkAction::Receive {
                    op,
                    from,
                    valid,
                    target: _,
                } => {
                    // BUG: technically we should wait for validation before
                    // receiving, but we don't currently check for that.
                    // we only require that the op is valid.
                    let any_integrated = nodes
                        .get(&from)
                        .ok_or(anyhow!("no node"))?
                        .iter_from(op.0)
                        .any(|(_, v)| {
                            matches!(
                                v,
                                OpFamilyPhase::Op(
                                    OpPhase::Integrated(Outcome::Accepted)
                                        | OpPhase::Validated(ValidationType::App)
                                )
                            )
                        });

                    // let any_integrated = nodes
                    //     .get(&from)
                    //     .ok_or(anyhow!("no node"))?
                    //     .all_integrated(op.0)
                    //     .any(|v| v.is_valid());

                    if !any_integrated {
                        bail!("can't receive op if target has not integrated")
                    }

                    if !valid {
                        bail!("invalid op not currently handled")
                    }

                    self.inner
                        .transition(node_state, (op, OpAction::Store(false).into()))
                }
            },
        )?;

        Ok((state, ()))
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
        ops: impl IntoIterator<Item = OpId<O, T>>,
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

impl<N: Id, O: Id, T: Id> Default for OpNetworkMachine<N, O, T> {
    fn default() -> Self {
        Self::new()
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, derive_more::From)]
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
        op: OpId<O, T>,
        action: OpFamilyAction<O>,
    },
    Receive {
        op: OpId<O, T>,
        from: N,
        valid: bool,
        target: OpSendTarget,
    },
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Exhaustive,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum OpSendTarget {
    Vault,
    Cache,
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

#[derive(Clone, PartialEq, Eq, Hash, Debug, derive_more::From)]
pub struct OpNetworkStatePretty<N: Id, O: Id, T: Id>(pub OpNetworkState<N, O, T>);

impl<N: Id, O: Id, T: Id> Display for OpNetworkStatePretty<N, O, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (n, node) in self.0.nodes.iter() {
            write!(f, "n{n} [ ")?;
            for (_, state) in node.iter() {
                write!(f, "{state}")?;
            }
            writeln!(f, " ]")?;
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, derive_more::From)]
pub struct OpNetworkEdgePretty<N: Id, O: Id, T: Id>(N, pub OpNetworkAction<N, O, T>);

impl<N: Id, O: Id, T: Id> Display for OpNetworkEdgePretty<N, O, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(node, action) = self;
        match action {
            OpNetworkAction::Local { op, action } => write!(f, "[{node}, {op}] {action:?}",)?,
            OpNetworkAction::Receive {
                op,
                from,
                valid,
                target,
            } => write!(f, "{node} ↢ {from}: Recv({op}, {valid}, {target})",)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use polestar::{diagram::exhaustive::write_dot_state_diagram_mapped, id::UpTo};

    use super::*;

    #[test]
    #[ignore = "diagram"]
    fn test_op_network_diagram() {
        use polestar::diagram::exhaustive::DiagramConfig;

        type N = UpTo<2>;
        type O = UpTo<2>;
        type T = UpTo<1>;

        let ns = N::all_values();
        let ops = <OpId<O, T>>::iter_exhaustive(None).collect_vec();

        // Create an instance of OpMachine with 1 dependency
        let machine: OpNetworkMachine<N, O, T> = OpNetworkMachine::new_bounded(ns, ops);
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
                ignore_loopbacks: true,
                ..Default::default()
            },
            |state| Some(OpNetworkStatePretty(state)),
            |(target, action)| Some(OpNetworkEdgePretty(target, action)),
        );
    }
}
