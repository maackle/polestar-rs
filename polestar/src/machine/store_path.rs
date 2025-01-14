//! A machine which wraps another machine, augmenting its state
//! with a history of all actions taken to reach that state.

use std::{fmt::Debug, hash::Hash};

use crate::prelude::*;

use derive_more::derive::Deref;

/// The state for [`StorePathMachine`]
#[derive(Deref, derive_more::Debug)]
pub struct StorePathState<S, A>
where
    S: Debug,
{
    /// The wrapped state of the inner machine
    #[deref]
    pub state: S,

    /// The history of all actions taken thus far
    #[debug(skip)]
    pub path: im::Vector<A>,
}

impl<S, A> StorePathState<S, A>
where
    S: Debug,
    A: Clone,
{
    /// Wrap a state in a [`StorePathState`], with an empty history
    pub fn new(state: S) -> Self {
        Self {
            state,
            path: im::Vector::new(),
        }
    }
}

impl<S, A> Clone for StorePathState<S, A>
where
    S: Clone + Debug,
    A: Clone,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            path: self.path.clone(),
        }
    }
}

// XXX: equality and hash ignore path! This is necessary for traversal to work well.
impl<S, A> PartialEq for StorePathState<S, A>
where
    S: Debug + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

impl<S, A> Eq for StorePathState<S, A> where S: Debug + Eq {}

impl<S, A> Hash for StorePathState<S, A>
where
    S: Debug + Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.state.hash(state);
    }
}

/// A machine which wraps another machine, augmenting its state
/// with a history of all actions taken to reach that state.
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
    M::Action: Clone + Debug,
{
    type State = StorePathState<M::State, M::Action>;
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
