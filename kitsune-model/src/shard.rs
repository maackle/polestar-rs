use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Display},
};

use anyhow::{anyhow, bail};
use exhaustive::Exhaustive;
use polestar::{ext::MapExt, id::Id, prelude::*};
use serde::{Deserialize, Serialize};

use crate::arq::Arq;

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
    Grow,
    Shrink,
    SyncNext,
}

//           █████               █████
//          ░░███               ░░███
//   █████  ███████    ██████   ███████    ██████
//  ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
// ░░█████   ░███      ███████   ░███    ░███████
//  ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
//  ██████   ░░█████ ░░████████  ░░█████ ░░██████
// ░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░

#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::From)]
pub struct ShardState {
    target_arq: Arq<u8>,
    synced_chunks: u8,
}

impl ShardState {
    pub fn new(arq: Arq<u8>) -> Self {
        Self {
            target_arq: arq,
            synced_chunks: 0,
        }
    }

    pub fn target_arq(&self) -> Arq<u8> {
        self.target_arq
    }

    pub fn synced_arq(&self) -> Arq<u8> {
        let mut a = self.target_arq;
        a.len = self.synced_chunks;
        a
    }
}

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
pub struct ShardMachine;

impl Machine for ShardMachine {
    type State = ShardState;
    type Action = ShardAction;
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        match action {
            ShardAction::Grow => {
                state.target_arq.len += 1;
            }
            ShardAction::Shrink => {
                state.target_arq.len -= 1;
            }
            ShardAction::SyncNext => {
                if state.synced_chunks < state.target_arq.len {
                    state.synced_chunks += 1;
                } else {
                    bail!("Shard is already synced");
                }
            }
        }
        Ok((state, ()))
    }

    fn is_terminal(&self, s: &Self::State) -> bool {
        s.synced_chunks == s.target_arq.len
    }
}

impl ShardMachine {}

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

    #[test]
    fn diagram_shard() {
        let arq = Arq::new(3, 0, 3);

        polestar::diagram::exhaustive::write_dot_state_diagram_mapped(
            "shard.dot",
            ShardMachine,
            ShardState::new(arq),
            &Default::default(),
            |state| state.synced_chunks,
            |action| action,
        );
    }
}
