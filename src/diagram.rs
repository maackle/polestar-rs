use petgraph::graph::DiGraph;

use crate::prelude::*;

pub mod exhaustive;
pub mod montecarlo;

pub fn to_dot<N, E>(graph: DiGraph<N, E>) -> String
where
    N: core::fmt::Debug,
    E: core::fmt::Debug,
{
    use petgraph::dot::Dot;
    format!("{:?}", Dot::with_config(&graph, &[]))
}

pub trait DiagramNode {
    type Node: Eq + std::hash::Hash + std::fmt::Debug;

    fn to_diagram_node(&self) -> Self::Node;
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use ::exhaustive::Exhaustive;
    use num_derive::{FromPrimitive, ToPrimitive};
    use num_traits::{FromPrimitive, ToPrimitive};
    use proptest_derive::Arbitrary;

    #[derive(
        Copy, Clone, Debug, PartialEq, Eq, FromPrimitive, ToPrimitive, Hash, derive_more::Display,
    )]
    enum Cycle {
        A,
        B,
        C,
        D,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Arbitrary, Exhaustive, ToPrimitive)]
    enum Turn {
        One = 1,
        Two = 2,
    }

    impl Machine for Cycle {
        type Action = Turn;
        type Fx = ();
        type Error = Infallible;

        fn transition(mut self, turn: Turn) -> Result<(Self, Self::Fx), Self::Error> {
            let n = turn.to_i8().unwrap();
            self = Cycle::from_i8((self.to_i8().unwrap() + n).rem_euclid(4)).unwrap();
            Ok((self, ()))
        }

        fn is_terminal(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_state_diagrams() {
        let (nodes_montecarlo, edges_montecarlo) = {
            use super::montecarlo::*;
            let config = DiagramConfig {
                steps: 10,
                walks: 10,
                ignore_loopbacks: false,
            };
            let graph1 = state_diagram(Cycle::D, &mut (), &config);
            let nodes1: HashSet<_> = graph1.node_weights().cloned().collect();
            let edges1: HashSet<_> = graph1.edge_weights().cloned().collect();

            let graph2 = state_diagram(Cycle::D, &mut (), &config);
            let nodes2: HashSet<_> = graph2.node_weights().cloned().collect();
            let edges2: HashSet<_> = graph2.edge_weights().cloned().collect();

            assert_eq!(nodes1, nodes2);
            assert_eq!(edges1, edges2);

            println!("{}", to_dot(graph1));

            (nodes1, edges1)
        };

        let (nodes_exhaustive, edges_exhaustive) = {
            use super::exhaustive::*;
            let config = DiagramConfig::default();

            let graph1 = state_diagram(Cycle::D, &config);
            let nodes1: HashSet<_> = graph1.node_weights().cloned().collect();
            let edges1: HashSet<_> = graph1.edge_weights().cloned().collect();

            let graph2 = state_diagram(Cycle::D, &config);
            let nodes2: HashSet<_> = graph2.node_weights().cloned().collect();
            let edges2: HashSet<_> = graph2.edge_weights().cloned().collect();

            assert_eq!(nodes1, nodes2);
            assert_eq!(edges1, edges2);

            println!("{}", to_dot(graph1));

            (nodes1, edges1)
        };

        assert_eq!(nodes_montecarlo, nodes_exhaustive);
        assert_eq!(edges_montecarlo, edges_exhaustive);
    }

    #[test]
    fn test_exhaustive_state_diagram() {
        use super::exhaustive::*;

        let config = DiagramConfig::default();

        let graph1 = state_diagram(Cycle::D, &config);
        let nodes1: HashSet<_> = graph1.node_weights().collect();
        let edges1: HashSet<_> = graph1.edge_weights().collect();

        let graph2 = state_diagram(Cycle::D, &config);
        let nodes2: HashSet<_> = graph2.node_weights().collect();
        let edges2: HashSet<_> = graph2.edge_weights().collect();

        assert_eq!(nodes1, nodes2);
        assert_eq!(edges1, edges2);
        println!("{}", to_dot(graph1));
    }
}
