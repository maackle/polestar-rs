//! The Dining Philosophers problem
//!
//! https://github.com/tlaplus/Examples/blob/master/specifications/State/State.tla
//!

#![allow(unused)]

use exhaustive::Exhaustive;
use polestar::{id::UpTo, prelude::*};

use itertools::Itertools;

const N: usize = 3;

type Id = UpTo<N, true>;

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
    Hunger,
    CleanUp,
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
                let can_eat = state.can_eat(p);
                let phil = &mut state.philosophers[*p];
                if phil.phase == Phase::Hungry && can_eat {
                    phil.phase = Phase::Eating;
                    state.forks.left_mut(p).clean = false;
                    state.forks.right_mut(p).clean = false;
                } else {
                    return Err(anyhow::anyhow!("cannot start eating"));
                }
            }
            Action::Think => {
                if state.philosophers[*p].phase == Phase::Eating {
                    state.philosophers[*p].phase = Phase::Thinking;
                } else {
                    return Err(anyhow::anyhow!("cannot think while hungry"));
                }
            }
            Action::Hunger => {
                if state.philosophers[*p].phase == Phase::Thinking {
                    state.philosophers[*p].phase = Phase::Hungry;
                } else {
                    return Err(anyhow::anyhow!("cannot get hungry while eating"));
                }
            }
            Action::CleanUp => {
                if state.philosophers[*p].phase != Phase::Eating {
                    let left = state.forks.left_mut(p);
                    if left.holder == p && !left.clean {
                        let h = p - 1;
                        left.holder = h;
                        left.clean = true;
                    }

                    let right = state.forks.right_mut(p);
                    if right.holder == p && !right.clean {
                        right.holder = p + 1;
                        right.clean = true;
                    }
                } else {
                    return Err(anyhow::anyhow!("cannot clean up while eating"));
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
                        holder: Id::wrapping(if i == 1 { 0 } else { i }),
                        clean: false,
                    })
                    .collect_vec()
                    .try_into()
                    .unwrap(),
            ),
            philosophers: Philosophers(
                (0..N)
                    .map(|_| Philosopher::default())
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
        &mut self.0[p.wrapping_sub(1)]
    }

    fn right(&mut self, p: Id) -> &mut Philosopher {
        &mut self.0[p.wrapping_add(1)]
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Philosopher {
    phase: Phase,
}

impl Philosopher {
    fn is_hungry(&self) -> bool {
        self.phase == Phase::Hungry
    }

    fn is_eating(&self) -> bool {
        self.phase == Phase::Eating
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum Phase {
    #[default]
    Hungry,
    Eating,
    Thinking,
}

/*█████                      █████
 ░░███                      ░░███
 ███████    ██████   █████  ███████    █████
░░░███░    ███░░███ ███░░  ░░░███░    ███░░
  ░███    ░███████ ░░█████   ░███    ░░█████
  ░███ ███░███░░░   ░░░░███  ░███ ███ ░░░░███
  ░░█████ ░░██████  ██████   ░░█████  ██████
   ░░░░░   ░░░░░░  ░░░░░░     ░░░░░  ░░░░░░*/

#[cfg(test)]
mod tests {
    use super::*;

    use polestar::diagram::write_dot;
    use polestar::logic::{conjoin, EvaluatePropositions, PropositionRegistry, Transition};
    use polestar::model_checker::ModelChecker;
    use polestar::util::product_exhaustive;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
    enum Prop {
        #[display("eating_{}", _0)]
        Eating(Id),

        #[display("hungry_{}", _0)]
        Hungry(Id),

        #[display("sharefork_{}_{}", _0, _1)]
        ShareFork(Id, Id),
    }

    impl EvaluatePropositions<Prop> for Transition<Model> {
        fn evaluate(&self, prop: &Prop) -> bool {
            let Transition(s, _, _) = self;
            let out = match *prop {
                Prop::Eating(p) => s.philosophers[*p].phase == Phase::Eating,
                Prop::Hungry(p) => s.philosophers[*p].phase == Phase::Hungry,
                Prop::ShareFork(p, q) => {
                    s.forks.left(p).holder == s.forks.right(q).holder
                        || s.forks.right(p).holder == s.forks.left(q).holder
                }
            };
            out
        }
    }

    #[test]
    fn model_check_dining_philosophers() {
        let model = Model;

        let mut props = PropositionRegistry::empty();

        let exclusive_access = conjoin(product_exhaustive::<Id, Id>().filter_map(|(p, q)| {
            if p != q {
                let sharefork = props.add(Prop::ShareFork(p, q)).unwrap();
                let eating_p = props.add(Prop::Eating(p)).unwrap();
                let eating_q = props.add(Prop::Eating(q)).unwrap();
                Some(format!("G( {sharefork} -> !({eating_p} && {eating_q}) )"))
            } else {
                None
            }
        }));

        let nobody_starves = conjoin(Id::iter_exhaustive(None).map(|p| {
            let hungry = props.add(Prop::Hungry(p)).unwrap();
            format!("G F !{hungry}")
        }));

        let ltl = conjoin([
            // no two philosophers can eat at the same time
            exclusive_access,
            // no philosopher can starve
            nobody_starves,
        ]);

        println!("LTL: {ltl}");

        let graph = model
            .clone()
            .traverse([State::default()])
            .ignore_loopbacks(true)
            .specced(props.clone(), &ltl)
            .unwrap()
            .diagram()
            .unwrap();
        let graph = graph.map(|_, s| format!("{s:?}"), |_, e| format!("{e:?}"));
        write_dot("dining-philosophers-mc.dot", &graph, &[]);

        model
            .traverse([State::default()])
            .ignore_loopbacks(true)
            .specced(props, &ltl)
            .unwrap()
            .model_check_report();
    }

    #[test]
    fn graph_dining_philosophers() {
        let initial = State::default();

        let graph = Model
            .traverse([State::default()])
            .ignore_loopbacks(true)
            .diagram()
            .unwrap();

        {
            let graph = graph.map(
                |_, s| {
                    Id::iter_exhaustive(None)
                        .map(|p| {
                            let mut m = String::new();

                            if s.forks.left(p).holder == p {
                                m.push_str(p.to_string().as_str());
                            } else {
                                m.push(' ');
                            }

                            m.push_str(if s.philosophers[*p].is_hungry() {
                                "P"
                            } else {
                                "p"
                            });

                            if s.forks.right(p).holder == p {
                                m.push_str((p + 1).to_string().as_str());
                            } else {
                                m.push(' ');
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
    }
}

mod implementation {}
