use crate::network::topology::Topology;
use crate::prelude::*;

use polestar::machine::Cog;
use polestar::prelude::*;

mod adjacency;
pub mod topology;

pub trait Node: Cog + Eq + Hash + 'static
// where
//     <Self::Model as Machine>::Action: Eq + Hash,
{
    type ID: Cog + Eq + Hash + Ord + Display;
    type Model: Machine<
        State = Self,
        Action = Self::Action,
        Fx = Vec<Effect<Self>>,
        Error = anyhow::Error,
    >;

    /// This is an unfortunate appendage needed to add extra trait bounds,
    /// it should be able to be inferred from the Self::Model type.
    type Action: Cog + Eq + Hash + 'static;
}

pub type ActionOf<N> = <<N as Node>::Model as Machine>::Action;

// /// Extract network effects from
// pub trait NetworkEffect<N: Node> {
//     fn network_effect(&self) -> Vec<Effect<N>>;
// }

pub enum Effect<N: Node> {
    Send { to: N::ID, action: ActionOf<N> },
}

/// Induce the specified node to take an action
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action<N: Node> {
    Node(N::ID, ActionOf<N>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State<N: Node> {
    pub nodes: BTreeMap<N::ID, N>,
}

pub struct Model<N: Node> {
    node_model: N::Model,
    topology: Topology<N::ID>,
}

impl<N: Node> polestar::Machine for Model<N> {
    type State = State<N>;
    type Action = Action<N>;
    type Fx = Vec<Effect<N>>;
    type Error = anyhow::Error;

    fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        tracing::info!("transition: {state:?} {action:?}");
        let Action::Node(node_id, action) = action;
        let fx = state
            .nodes
            .owned_update(node_id, |_, node| self.node_model.transition(node, action))?;
        Ok((state, fx))
    }
}

impl<N: Node> Model<N> {
    /// Transition a node and trigger any follow-on actions due to effects.
    pub fn transition_recursively(
        &self,
        mut state: State<N>,
        action: Action<N>,
    ) -> Result<(State<N>, Vec<(N::ID, Action<N>)>), anyhow::Error> {
        let mut actions_applied = vec![];
        match action {
            Action::Node(node_id, action) => {
                let mut fx: VecDeque<(N::ID, Effect<N>)> = [(
                    node_id.clone(),
                    Effect::Send {
                        to: node_id,
                        action,
                    },
                )]
                .into_iter()
                .collect();

                loop {
                    if fx.is_empty() {
                        break;
                    }

                    let (sender, effect) = fx.pop_front().unwrap();
                    match effect {
                        Effect::Send {
                            to: receiver,
                            action,
                        } => {
                            if self.topology.has_edge(sender.clone(), receiver.clone()) {
                                let action = Action::Node(receiver.clone(), action);
                                let (s, e) = self.transition(state, action.clone())?;
                                state = s;
                                actions_applied.push((sender.clone(), action));
                                fx.extend(e.into_iter().map(|e| (receiver.clone(), e)));
                            }
                        }
                    }
                }
            }
        }
        Ok((state, actions_applied))
    }
}

#[cfg(test)]
mod tests {
    use crate::network::topology::Topology;

    use super::*;

    type Id = u8;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TestNode {
        me: Id,
        peers: BTreeSet<Id>,
        sum: u32,
    }

    impl Node for TestNode {
        type ID = Id;
        type Action = u32;
        type Model = TestModel;
    }

    struct TestModel;

    impl Machine for TestModel {
        type State = TestNode;
        type Action = u32;
        type Fx = Vec<Effect<TestNode>>;
        type Error = anyhow::Error;

        fn transition(
            &self,
            mut state: Self::State,
            action: Self::Action,
        ) -> TransitionResult<Self> {
            state.sum += action;
            let fx = if action > 1 {
                state
                    .peers
                    .iter()
                    .map(|p| Effect::Send {
                        to: *p,
                        action: action - 1,
                    })
                    .collect()
            } else {
                vec![]
            };
            Ok((state, fx))
        }
    }

    #[test]
    fn test_transition_recursively() {
        let topology = Topology::FullyConnected;
        let model = Model::<TestNode> {
            node_model: TestModel,
            topology,
        };
        let nodes = (0..3)
            .map(|i| {
                (
                    i,
                    TestNode {
                        me: i,
                        peers: (0..3).filter(|j| *j != i).collect(),
                        sum: 0,
                    },
                )
            })
            .collect();
        let state = State { nodes };
        let action = Action::Node(0, 3);
        let (result, actions) = model.transition_recursively(state, action).unwrap();
        assert_eq!(actions.len(), 7);
        assert_eq!(result.nodes[&0].sum, 5);
        assert_eq!(result.nodes[&1].sum, 3);
        assert_eq!(result.nodes[&2].sum, 3);
    }
}
