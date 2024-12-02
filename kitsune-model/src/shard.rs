use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Display},
};

use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use polestar::{ext::MapExt, id::Id, prelude::*};
use serde::{Deserialize, Serialize};

//                      █████     ███
//                     ░░███     ░░░
//   ██████    ██████  ███████   ████   ██████  ████████
//  ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
//   ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
//  ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
// ░░████████░░██████   ░░█████  █████░░██████  ████ █████
//  ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Exhaustive, Serialize, Deserialize)]
pub enum ShardAction {
    Noop,
}

//           █████               █████
//          ░░███               ░░███
//   █████  ███████    ██████   ███████    ██████
//  ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
// ░░█████   ░███      ███████   ░███    ░███████
//  ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
//  ██████   ░░█████ ░░████████  ░░█████ ░░██████
// ░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, derive_more::From)]
pub struct ShardState {}

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
pub struct ShardMachine {}

impl Machine for ShardMachine {
    type State = ShardState;
    type Action = ShardAction;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        Ok((state, ()))
    }

    fn is_terminal(&self, s: &Self::State) -> bool {
        false
    }
}

impl ShardMachine {
    pub fn new() -> Self {
        todo!()
    }

    pub fn initial(&self) -> ShardState {
        todo!()
    }
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

#[cfg(test)]
mod tests {

    use super::*;
}
