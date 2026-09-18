//! Deprecated state-diagram generators, superseded by
//! [`Traversal::diagram`](polestar_model_checker::traversal::Traversal::diagram).

#[deprecated = "use traversal with graphing enabled instead"]
pub mod exhaustive;
#[deprecated = "use traversal with graphing enabled instead"]
pub mod montecarlo;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use ::exhaustive::Exhaustive;
    use num_derive::{FromPrimitive, ToPrimitive};
    use num_traits::{FromPrimitive, ToPrimitive};
    use polestar_core::prelude::*;
    use polestar_model_checker::{diagram::to_dot, prelude::MachineExt};
    use proptest_derive::Arbitrary;

    #[derive(Clone)]
    struct CycleMachine;

    #[derive(
        Copy, Clone, Debug, PartialEq, Eq, FromPrimitive, ToPrimitive, Hash, derive_more::Display,
    )]
    enum Cycle {
        A,
        B,
        C,
        D,
    }

    #[derive(
        Copy,
        Clone,
        Debug,
        PartialEq,
        Eq,
        Hash,
        Arbitrary,
        Exhaustive,
        ToPrimitive,
        derive_more::Display,
    )]
    enum Turn {
        One = 1,
        Two = 2,
    }

    impl Machine for CycleMachine {
        type State = Cycle;
        type Action = Turn;
        type Fx = ();
        type Error = Infallible;

        fn transition(
            &self,
            mut state: Self::State,
            turn: Turn,
        ) -> Result<(Self::State, Self::Fx), Self::Error> {
            let n = turn.to_i8().unwrap();
            state = Cycle::from_i8((state.to_i8().unwrap() + n).rem_euclid(4)).unwrap();
            Ok((state, ()))
        }

        fn is_terminal(&self, _: &Self::State) -> bool {
            false
        }
    }

    #[test]
    #[allow(deprecated)]
    fn test_state_diagrams() {
        let (nodes_montecarlo, edges_montecarlo) = {
            use super::montecarlo::*;
            let config = DiagramConfig {
                steps: 10,
                walks: 10,
                ignore_loopbacks: false,
            };
            let graph1 = state_diagram(CycleMachine, Cycle::D, &mut (), &config);
            let nodes1: HashSet<_> = graph1.node_weights().cloned().collect();
            let edges1: HashSet<_> = graph1.edge_weights().cloned().collect();

            let graph2 = state_diagram(CycleMachine, Cycle::D, &mut (), &config);
            let nodes2: HashSet<_> = graph2.node_weights().cloned().collect();
            let edges2: HashSet<_> = graph2.edge_weights().cloned().collect();

            assert_eq!(nodes1, nodes2);
            assert_eq!(edges1, edges2);

            println!("{}", to_dot(&graph1, &[]));

            (nodes1, edges1)
        };

        let (nodes_exhaustive, edges_exhaustive) = {
            use super::exhaustive::*;
            let config = DiagramConfig::default();

            let graph1 = state_diagram(CycleMachine, Cycle::D, &config);
            let nodes1: HashSet<_> = graph1.node_weights().cloned().collect();
            let edges1: HashSet<_> = graph1.edge_weights().cloned().collect();

            let graph2 = state_diagram(CycleMachine, Cycle::D, &config);
            let nodes2: HashSet<_> = graph2.node_weights().cloned().collect();
            let edges2: HashSet<_> = graph2.edge_weights().cloned().collect();

            assert_eq!(nodes1, nodes2);
            assert_eq!(edges1, edges2);

            println!("{}", to_dot(&graph1, &[]));

            (nodes1, edges1)
        };

        let (nodes_traversal, edges_traversal) = {
            let graph = CycleMachine.traverse([Cycle::D]).diagram().unwrap();
            let nodes: HashSet<_> = graph.node_weights().cloned().collect();
            let edges: HashSet<_> = graph.edge_weights().cloned().collect();

            println!("{}", to_dot(&graph, &[]));

            (nodes, edges)
        };

        assert_eq!(nodes_montecarlo, nodes_exhaustive);
        assert_eq!(edges_montecarlo, edges_exhaustive);
        assert_eq!(nodes_exhaustive, nodes_traversal);
        assert_eq!(edges_exhaustive, edges_traversal);
    }
}
