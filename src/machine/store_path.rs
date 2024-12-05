use crate::prelude::*;

use derive_more::derive::Deref;
use tracing_subscriber::field::debug;

#[derive(Deref, derive_more::Debug)]
pub struct StorePathState<M>
where
    M: Machine,
    M::State: std::fmt::Debug,
{
    #[deref]
    pub state: M::State,
    #[debug(skip)]
    pub path: im::Vector<M::Action>,
}

impl<M> StorePathState<M>
where
    M: Machine,
    M::State: std::fmt::Debug,
    M::Action: Clone,
{
    pub fn new(state: M::State) -> Self {
        Self {
            state,
            path: im::Vector::new(),
        }
    }
}

impl<M> Clone for StorePathState<M>
where
    M: Machine,
    M::State: Clone + std::fmt::Debug,
    M::Action: Clone,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            path: self.path.clone(),
        }
    }
}

#[derive(Clone, Debug, Deref, derive_more::From)]
pub struct StorePathMachine<M>
where
    M: Machine,
{
    machine: M,
}

impl<M> Machine for StorePathMachine<M>
where
    M: Machine,
    M::State: std::fmt::Debug,
    M::Action: Clone,
{
    type State = StorePathState<M>;
    type Action = M::Action;
    type Error = M::Error;
    type Fx = M::Fx;

    fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        let (next, fx) = self.machine.transition(state.state, action.clone())?;
        state.state = next;
        state.path.push_back(action);
        Ok((state, fx))
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        self.machine.is_terminal(&state.state)
    }
}
