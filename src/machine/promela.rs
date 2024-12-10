pub mod parser;

use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};

use anyhow::anyhow;
use im::Vector;
use parser::*;

use super::{
    store_path::{StorePathMachine, StorePathState},
    Machine, TransitionResult,
};

#[derive(derive_bounded::Debug)]
#[bounded_to(M::Error, M::Action)]
pub enum BuchiError<M>
where
    M: Machine,
    M::Action: Debug + Clone,
    M::Error: Debug,
{
    Internal(anyhow::Error),
    LtlError {
        error: anyhow::Error,
        path: Vector<M::Action>,
        state: Arc<BuchiState>,
    },
    MachineError(M::Error),
}

pub struct PromelaMachine<M>
where
    M: Machine,
{
    buchi: PromelaBuchi,
    machine: StorePathMachine<M>,
}

impl<M> Machine for PromelaMachine<M>
where
    M: Machine,
    M::State: Propositions + Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    type State = PromelaState<M>;
    type Action = M::Action;
    type Error = BuchiError<M>;
    type Fx = M::Fx;

    fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        let PromelaState { state, buchi } = state;

        // // If the buchi state is empty, it is all-accepting, and we never have to check it again.
        // if buchi.is_empty() {
        //     return self
        //         .machine
        //         .transition(state, action)
        //         .map(|(next, fx)| (PromelaState { state: next, buchi }, fx))
        //         .map_err(BuchiError::MachineError);
        // }

        match &*buchi {
            BuchiState::AcceptAll => {
                let (next, fx) = self
                    .machine
                    .transition(state, action)
                    .map_err(BuchiError::MachineError)?;
                Ok((PromelaState { state: next, buchi }, fx))
            }
            BuchiState::Predicates(predicates) => {
                if let Some((_, next_state_name)) =
                    predicates.iter().find(|(ltl, _)| ltl.eval(&state.state))
                {
                    let (next, fx) = self
                        .machine
                        .transition(state, action)
                        .map_err(BuchiError::MachineError)?;
                    let buchi_next = self
                        .buchi
                        .states
                        .get(next_state_name)
                        .ok_or_else(|| {
                            BuchiError::Internal(anyhow!(
                                "no buchi state named '{next_state_name}'. This is a polestar bug."
                            ))
                        })?
                        .clone();
                    let next = PromelaState {
                        state: next,
                        buchi: buchi_next,
                    };
                    Ok((next, fx))
                } else {
                    Err(BuchiError::LtlError {
                        error: anyhow!("LTL not satisfied"),
                        state: buchi.clone(),
                        path: state.path,
                    })
                }
            }
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        self.machine.is_terminal(&state.state)
    }
}

impl<M> PromelaMachine<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    pub fn new(machine: M, ltl: &str) -> Self {
        let buchi = PromelaBuchi::from_ltl(ltl);
        Self {
            buchi,
            machine: StorePathMachine::from(machine),
        }
    }

    pub fn initial(&self, state: M::State) -> PromelaState<M> {
        let buchi = self
            .buchi
            .states
            .get("accept_init")
            .or_else(|| self.buchi.states.get("T0_init"))
            .unwrap()
            .clone();
        PromelaState::new(state, buchi)
    }
}

#[derive(
    Debug, derive_bounded::Clone, derive_bounded::PartialEq, derive_bounded::Eq, derive_more::Deref,
)]
#[bounded_to(StorePathState<M>)]
pub struct PromelaState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    #[deref]
    state: StorePathState<M>,
    buchi: Arc<BuchiState>,
}

impl<M> Hash for PromelaState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.state.hash(state);
    }
}

impl<M> PromelaState<M>
where
    M: Machine,
    M::State: Clone + Debug + Eq + Hash,
    M::Action: Clone + Debug,
{
    pub fn new(state: M::State, buchi_state: Arc<BuchiState>) -> Self {
        Self {
            state: StorePathState::new(state),
            buchi: buchi_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        id::UpTo,
        traversal::{traverse, TraversalConfig},
    };

    use super::*;
    use crate::diagram::exhaustive::*;

    #[derive(Debug, Hash, PartialEq, Eq)]
    struct TestMachine;

    impl Machine for TestMachine {
        type State = u8;
        type Action = UpTo<4>;

        fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self> {
            // Ok((state.wrapping_mul(*action as u8), ()))

            Ok((state.wrapping_add(2), ()))
        }

        fn is_terminal(&self, _: &Self::State) -> bool {
            false
        }
    }

    impl Propositions for u8 {
        fn eval(&self, p: &str) -> bool {
            match p {
                "even" => self % 2 == 0,
                "max" => *self == 255,
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn promela_test() {
        let machine = PromelaMachine::new(TestMachine, "F max");
        let initial = machine.initial(0);

        let (report, terminals) = traverse(
            machine,
            initial,
            &TraversalConfig {
                record_terminals: true,
                ..Default::default()
            }
            .with_fatal_error(|e| matches!(e, BuchiError::LtlError { .. })),
            Some,
        )
        .unwrap();
        dbg!(&report);

        let (terminals, loop_terminals) = terminals.unwrap();
        for t in loop_terminals.into_iter() {
            dbg!(&t.state.path.len());
        }
    }

    #[test]
    #[ignore = "diagram"]
    fn promela_diagram() {
        write_dot_state_diagram_mapped(
            "promela-test.dot",
            TestMachine,
            1,
            &DiagramConfig {
                max_depth: None,
                ..Default::default()
            },
            Some,
            Some,
        );
    }
}
