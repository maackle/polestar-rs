//! The Dining Philosophers problem
//!
//! https://github.com/tlaplus/Examples/blob/master/specifications/State/State.tla
//!

use exhaustive::Exhaustive;
use polestar::{
    id::UpTo,
    prelude::*,
    traversal::{traverse, TraversalConfig, TraversalGraphingConfig},
};

use itertools::Itertools;

const N: usize = 3;

type Id = UpTo<N>;

/*                   █████     ███
                    ░░███     ░░░
  ██████    ██████  ███████   ████   ██████  ████████
 ░░░░░███  ███░░███░░░███░   ░░███  ███░░███░░███░░███
  ███████ ░███ ░░░   ░███     ░███ ░███ ░███ ░███ ░███
 ███░░███ ░███  ███  ░███ ███ ░███ ░███ ░███ ░███ ░███
░░████████░░██████   ░░█████  █████░░██████  ████ █████
 ░░░░░░░░  ░░░░░░     ░░░░░  ░░░░░  ░░░░░░  ░░░░ ░░░░░   */

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, exhaustive::Exhaustive)]
pub enum Action {
    Eat,
    Think,
    Interact,
}

/*                                  █████       ███
                                   ░░███       ░░░
 █████████████    ██████    ██████  ░███████   ████  ████████    ██████
░░███░░███░░███  ░░░░░███  ███░░███ ░███░░███ ░░███ ░░███░░███  ███░░███
 ░███ ░███ ░███   ███████ ░███ ░░░  ░███ ░███  ░███  ░███ ░███ ░███████
 ░███ ░███ ░███  ███░░███ ░███  ███ ░███ ░███  ░███  ░███ ░███ ░███░░░
 █████░███ █████░░████████░░██████  ████ █████ █████ ████ █████░░██████
░░░░░ ░░░ ░░░░░  ░░░░░░░░  ░░░░░░  ░░░░ ░░░░░ ░░░░░ ░░░░ ░░░░░  ░░░░░░  */

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Model;

impl Machine for Model {
    type State = State;
    type Action = (Id, Action);
    type Fx = ();
    type Error = anyhow::Error;

    fn transition(
        &self,
        mut state: Self::State,
        (p, action): Self::Action,
    ) -> TransitionResult<Self> {
        match action {
            Action::Eat => {
                if state.philosophers[*p].hungry && state.can_eat(p) {
                    state.philosophers[*p].hungry = false;
                    state.forks.left_mut(p).clean = false;
                    state.forks.right_mut(p).clean = false;
                }
            }
            Action::Think => {
                state.philosophers[*p].hungry = true;
            }
            Action::Interact => {
                let left = state.forks.left_mut(p);
                if left.holder == p && !left.clean {
                    left.holder = p - 1;
                    left.clean = true;
                }

                let right = state.forks.right_mut(p);
                if right.holder == p && !right.clean {
                    right.holder = p + 1;
                    right.clean = true;
                }
            }
        }
        Ok((state, ()))
    }

    fn is_terminal(&self, s: &Self::State) -> bool {
        false
    }
}

/*        █████               █████
         ░░███               ░░███
  █████  ███████    ██████   ███████    ██████
 ███░░  ░░░███░    ░░░░░███ ░░░███░    ███░░███
░░█████   ░███      ███████   ░███    ░███████
 ░░░░███  ░███ ███ ███░░███   ░███ ███░███░░░
 ██████   ░░█████ ░░████████  ░░█████ ░░██████
░░░░░░     ░░░░░   ░░░░░░░░    ░░░░░   ░░░░░░  */

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct State {
    forks: Forks,
    philosophers: Philosophers,
}

impl Default for State {
    fn default() -> Self {
        Self {
            forks: Forks(
                (0..N)
                    .map(|i| Fork {
                        holder: Id::new(if i == 1 { 0 } else { i }),
                        clean: false,
                    })
                    .collect_vec()
                    .try_into()
                    .unwrap(),
            ),
            philosophers: Philosophers(
                (0..N)
                    .map(|_| Philosopher { hungry: true })
                    .collect_vec()
                    .try_into()
                    .unwrap(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::Deref, derive_more::DerefMut)]
pub struct Forks([Fork; N]);

impl Forks {
    fn left(&self, p: Id) -> &Fork {
        &self.0[*p]
    }

    fn left_mut(&mut self, p: Id) -> &mut Fork {
        &mut self.0[*p]
    }

    fn right(&self, p: Id) -> &Fork {
        &self.0[*(p + 1)]
    }

    fn right_mut(&mut self, p: Id) -> &mut Fork {
        &mut self.0[*(p + 1)]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::Deref, derive_more::DerefMut)]
pub struct Philosophers([Philosopher; N]);

impl Philosophers {
    fn left(&mut self, p: Id) -> &mut Philosopher {
        &mut self.0[*(p - 1)]
    }

    fn right(&mut self, p: Id) -> &mut Philosopher {
        &mut self.0[*(p + 1)]
    }
}

impl State {
    fn is_holding_both_forks(&self, p: Id) -> bool {
        self.forks.left(p).holder == p && self.forks.right(p).holder == p
    }

    fn both_forks_are_clean(&self, p: Id) -> bool {
        self.forks.left(p).clean && self.forks.right(p).clean
    }

    fn can_eat(&self, p: Id) -> bool {
        self.is_holding_both_forks(p) && self.both_forks_are_clean(p)
    }
}

struct Sides<T> {
    left: T,
    right: T,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fork {
    holder: Id,
    clean: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Philosopher {
    hungry: bool,
}

/*█████                      █████
 ░░███                      ░░███
 ███████    ██████   █████  ███████    █████
░░░███░    ███░░███ ███░░  ░░░███░    ███░░
  ░███    ░███████ ░░█████   ░███    ░░█████
  ░███ ███░███░░░   ░░░░███  ░███ ███ ░░░░███
  ░░█████ ░░██████  ██████   ░░█████  ██████
   ░░░░░   ░░░░░░  ░░░░░░     ░░░░░  ░░░░░░*/

#[test]
fn test_dining_philosophers() {
    let dining_philosophers = State::default();

    let traversal_config = TraversalConfig::builder()
        .graphing(TraversalGraphingConfig {
            ignore_loopbacks: true,
            ..Default::default()
        })
        .build();

    let (report, graph, _) =
        traverse::<Model, State>(Model.into(), dining_philosophers, traversal_config, Some)
            .unwrap();

    {
        let graph = graph.unwrap().map(
            |_, s| {
                Id::iter_exhaustive(None)
                    .map(|p| {
                        let mut m = String::new();

                        if s.forks.left(p).holder == p {
                            m.push_str(p.to_string().as_str());
                        } else {
                            m.push_str(" ");
                        }

                        m.push_str(if s.philosophers[*p].hungry { "P" } else { "p" });

                        if s.forks.right(p).holder == p {
                            m.push_str((p + 1).to_string().as_str());
                        } else {
                            m.push_str(" ");
                        }

                        m
                    })
                    .join(" ")
            },
            |_, (p, a)| format!("{p}:{a:?}"),
        );
        polestar::diagram::write_dot(
            "dining-philosophers.dot",
            &graph,
            // &[petgraph::dot::Config::EdgeNoLabel],
            &[],
        );
    }

    println!("{:#?}", report);
}
