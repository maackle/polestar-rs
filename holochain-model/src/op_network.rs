use std::collections::{BTreeMap, BTreeSet};

use anyhow::anyhow;
use exhaustive::Exhaustive;
use polestar::{id::Id, Machine, TransitionResult};

use crate::{
    op_family::{OpFamilyAction, OpFamilyMachine, OpFamilyState},
    op_single::{OpAction, ValidationType},
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::Constructor)]
pub struct OpNetworkMachine<N: Id, O: Id> {
    pub inner: OpFamilyMachine<O>,
    pub nodes: BTreeSet<N>,
}

impl<N: Id, O: Id> Machine for OpNetworkMachine<N, O> {
    type State = OpNetworkState<N, O>;
    type Action = (N, OpNetworkAction<N, O>);

    fn transition(
        &self,
        state: Self::State,
        (node, action): Self::Action,
    ) -> TransitionResult<Self> {
        todo!();

        let node_state = state
            .nodes
            .remove(&node)
            .ok_or(anyhow!("no node {:?}", node))?;
    }

    fn is_terminal(&self, _: &Self::State) -> bool {
        false
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
    Family(OpFamilyAction<O>),
    Receive { op: O, from: N, valid: bool },
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
