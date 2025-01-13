//! A "fun" little example of solving the best way to spam emojis
//! using a phone keyboard.
//!
//! The use case is the following:
//! - You want to send a long string of a single emoji to a friend using your phone keyboard.
//! - Your strategy is to type a few emojis, copy them, and then paste them over and over.
//! - Every once in a while, you want to "Select All" and copy again, to increase your pasting power.
//!
//! The problem we're solving is, how can you reach a target number of emojis
//! with the minimal effort?
//!
//! "Effort" is defined in terms of a cost function [`SpamState::cost`], where each Action has a cost.
//! - Typing a single character is the lowest cost action.
//! - Pasting the clipboard is slightly more laborious.
//! - "Select All" + "Paste" is the most laborious action.

use anyhow::bail;
use exhaustive::Exhaustive;
use polestar::{
    machine::store_path::{StorePathMachine, StorePathState},
    prelude::*,
    traversal::Traversal,
};

fn main() {
    // tracing_subscriber::fmt::fmt()
    //     .with_max_level(tracing::Level::DEBUG)
    //     .init();

    let target = 1_000;

    let machine = StorePathMachine::from(SpamMachine { target });
    let initial = StorePathState::new(SpamState::default());
    let terminals = Traversal::new(machine, [initial])
        .max_depth(50)
        .run_terminal();

    let terminals = terminals.unwrap();
    let mut states: Vec<_> = terminals.into_iter().filter(|s| s.len >= target).collect();
    states.sort_by_key(|s| s.cost);
    let min_cost = states[0].cost;

    let mut solutions: Vec<_> = states
        .into_iter()
        .take_while(|s| s.cost <= min_cost + 3)
        .take(10)
        .collect();
    solutions.sort_by_key(|s| (s.cost, s.len));

    for (i, state) in solutions.into_iter().enumerate() {
        let path = state
            .path
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join("");
        println!(
            "#{:<2} : len={:<6} cost={:<3} path={}",
            i + 1,
            state.len,
            state.cost,
            path
        );
    }
}

struct SpamMachine {
    target: usize,
}

impl Machine for SpamMachine {
    type Action = SpamAction;
    type State = SpamState;
    type Error = anyhow::Error;
    type Fx = ();

    fn is_terminal(&self, s: &Self::State) -> bool {
        s.len >= self.target
    }

    fn transition(&self, mut s: Self::State, a: Self::Action) -> TransitionResult<Self> {
        match a {
            SpamAction::One => {
                if s.pastes.is_some() {
                    bail!("don't do Ones if you've already copied");
                }
                s.len += 1;
            }
            SpamAction::Copy => {
                if s.pastes == Some(0) {
                    bail!("already copied once");
                }
                if s.len == 0 {
                    bail!("no length to copy");
                }
                s.buf = Some(s.len);
                s.pastes = Some(0);
            }
            SpamAction::Paste => {
                if let Some(buf) = s.buf {
                    s.len += buf;
                } else {
                    bail!("no buffer");
                }
                *s.pastes.as_mut().unwrap() += 1
            }
        }
        s.cost += s.cost(&a);
        Ok((s, ()))
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Exhaustive, derive_more::Debug, derive_more::Display)]
enum SpamAction {
    #[display("1")]
    #[debug("1")]
    /// Type a single emoji.    
    One,
    #[display("C")]
    #[debug("C")]
    /// Select all and copy.
    Copy,
    #[display("P")]
    #[debug("P")]
    /// Paste the currently selected text.
    Paste,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
struct SpamState {
    /// The number of emojis currently onscreen.
    len: usize,
    /// The cost of the actions taken so far.
    cost: usize,
    /// The paste buffer length.
    buf: Option<usize>,
    /// The number of times the buffer has been pasted since the last copy.
    pastes: Option<usize>,
}

impl SpamState {
    fn cost(&self, action: &SpamAction) -> usize {
        match action {
            SpamAction::One => 1,
            SpamAction::Copy => 5,
            SpamAction::Paste => 3,
        }
    }
}
