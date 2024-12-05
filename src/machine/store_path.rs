use std::{fmt::Debug, hash::Hash};

use crate::prelude::*;

use derive_more::derive::Deref;

#[derive(Deref, derive_more::Debug)]
pub struct StorePathState<M>
where
    M: Machine,
    M::State: Debug,
{
    #[deref]
    pub state: M::State,
    #[debug(skip)]
    pub path: im::Vector<M::Action>,
}

impl<M> StorePathState<M>
where
    M: Machine,
    M::State: Debug,
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
    M::State: Clone + Debug,
    M::Action: Clone,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            path: self.path.clone(),
        }
    }
}

// XXX: equality and hash ignore path! This is necessary for traversal to work well.
impl<M> PartialEq for StorePathState<M>
where
    M: Machine,
    M::State: Debug + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

impl<M> Eq for StorePathState<M>
where
    M: Machine,
    M::State: Debug + Eq,
{
}

impl<M> Hash for StorePathState<M>
where
    M: Machine,
    M::State: Debug + Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.state.hash(state);
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
    M::State: Debug,
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
