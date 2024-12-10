pub mod parser;

use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use parser::*;

use super::{Machine, TransitionResult};

#[derive(Debug)]
pub enum BuchiError<E> {
    Buchi(anyhow::Error),
    MachineError(E),
}

pub struct PromelaMachine<M>
where
    M: Machine,
{
    buchi: PromelaBuchi,
    machine: M,
}

impl<M> Machine for PromelaMachine<M>
where
    M: Machine,
    M::State: Propositions + Clone,
{
    type State = PromelaState<M>;
    type Action = M::Action;
    type Error = BuchiError<M::Error>;
    type Fx = M::Fx;

    fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        let PromelaState(state, buchi_state) = state;
        if let Some((_, next_state_name)) = buchi_state.iter().find(|(ltl, _)| ltl.eval(&state)) {
            let (next, fx) = self
                .machine
                .transition(state, action)
                .map_err(BuchiError::MachineError)?;
            let buchi_next = self
                .buchi
                .states
                .get(next_state_name)
                .ok_or(BuchiError::Buchi(anyhow!(
                    "no buchi state '{next_state_name}'"
                )))?
                .clone();
            let next = PromelaState(next, buchi_next);
            Ok((next, fx))
        } else {
            Err(BuchiError::Buchi(anyhow!("no buchi transition found")))
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        self.machine.is_terminal(&state.0)
    }
}

impl<M> PromelaMachine<M>
where
    M: Machine,
    M::State: Clone,
{
    pub fn new(machine: M, ltl: &str) -> Self {
        let buchi = PromelaBuchi::from_ltl(ltl);
        Self { buchi, machine }
    }

    pub fn initial(&self, state: M::State) -> PromelaState<M> {
        let buchi_init = self
            .buchi
            .states
            .get("accept_init")
            .or_else(|| self.buchi.states.get("T0_init"))
            .unwrap()
            .clone();
        PromelaState(state, buchi_init)
    }
}

#[derive(Debug, derive_bounded::Clone, derive_bounded::PartialEq, derive_bounded::Eq, Hash)]
#[bounded_to(M::State)]
pub struct PromelaState<M>(M::State, Arc<BuchiState>)
where
    M: Machine,
    M::State: Clone;

impl<M> PromelaState<M>
where
    M: Machine,
    M::State: Clone,
{
    pub fn new(state: M::State, buchi_state: Arc<BuchiState>) -> Self {
        Self(state, buchi_state)
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
            Ok((state.wrapping_mul(*action as u8), ()))
        }

        fn is_terminal(&self, _: &Self::State) -> bool {
            false
        }
    }

    impl Propositions for u8 {
        fn eval(&self, p: &str) -> bool {
            match p {
                "even" => self % 2 == 0,
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn promela_test() {
        let machine = PromelaMachine::new(TestMachine, "F even");
        let initial = machine.initial(1);

        let (report, _) = traverse(
            machine,
            initial,
            &TraversalConfig {
                ..Default::default()
            }
            .with_fatal_error(|e| matches!(e, BuchiError::Buchi(_))),
            Some,
        )
        .unwrap();
        dbg!(&report);
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
