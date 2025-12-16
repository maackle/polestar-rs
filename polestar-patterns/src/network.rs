use crate::{network::adjacency::Adjacency, prelude::*};

use polestar::prelude::*;
use polestar::{machine::Cog, *};

mod adjacency;

pub trait Node: Cog + 'static {
    type ID: Cog + Ord + Display;
    type Action: Cog;
    type Model: Machine<
        State = Self,
        Action = Self::Action,
        Fx = Option<Effect<Self>>,
        Error = anyhow::Error,
    >;
}

// /// Extract network effects from
// pub trait NetworkEffect<N: Node> {
//     fn network_effect(&self) -> Option<Effect<N>>;
// }

pub enum Effect<N: Node> {
    Send { to: N::ID, action: N::Action },
}

/// Induce the specified node to take an action
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action<N: Node> {
    Node(N::ID, N::Action),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State<N: Node> {
    pub nodes: BTreeMap<N::ID, N>,
}

impl<N: Node> Default for State<N> {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }
}

pub struct Model<N: Node> {
    node_model: N::Model,
    adjacency: Adjacency<N::ID>,
}

impl<N: Node> polestar::Machine for Model<N> {
    type State = State<N>;
    type Action = Action<N>;
    type Fx = Option<Effect<N>>;
    type Error = anyhow::Error;

    fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        let Action::Node(node_id, action) = action;
        let fx = state
            .nodes
            .owned_update(node_id, |_, node| self.node_model.transition(node, action))?;
        Ok((state, fx))
    }
}

impl<N: Node> Model<N> {
    pub fn connectivity(&self, a: N::ID, b: N::ID) -> bool {
        self.adjacency.has_path_between(a, b)
    }
}
