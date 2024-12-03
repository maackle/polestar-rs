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
//! "Effort" is defined in terms of a cost function [`SpamState::cost`], where each Action has a cost.

use anyhow::bail;
use exhaustive::Exhaustive;
use polestar::prelude::*;

struct SpamMachine {
    target: usize,
}

impl Machine for SpamMachine {
    type Action = SpamAction;
    type State = SpamState;

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

fn main() {
    // tracing_subscriber::fmt::fmt()
    //     .with_max_level(tracing::Level::DEBUG)
    //     .init();

    let target = 1_000;

    let machine = SpamMachine { target };
    let initial = SpamState::default();

    let (states, _report) = polestar::traversal::traverse(
        machine,
        initial,
        &polestar::traversal::TraversalConfig {
            max_depth: Some(50),
            ..Default::default()
        },
    )
    .unwrap();

    dbg!(&states.len());
    let mut states: Vec<_> = states
        .into_iter()
        .filter(|(s, _)| s.len >= target)
        .collect();
    states.sort_by_key(|(s, _)| s.cost);
    let min_cost = states[0].0.cost;

    let mut solutions: Vec<_> = states
        .into_iter()
        .take_while(|(s, _)| s.cost <= min_cost + 3)
        .take(10)
        .collect();
    solutions.sort_by_key(|(s, _)| (s.cost, s.len));

    for (i, (state, path)) in solutions.into_iter().enumerate() {
        let path = path
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
