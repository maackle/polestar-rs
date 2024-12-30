use petgraph::graph::DiGraph;

use crate::prelude::*;

#[deprecated = "use traversal with graphing enabled instead"]
pub mod exhaustive;
#[deprecated = "use traversal with graphing enabled instead"]
pub mod montecarlo;

pub fn write_dot<N, E>(filename: &str, graph: &DiGraph<N, E>, config: &[petgraph::dot::Config])
where
    N: core::fmt::Display,
    E: core::fmt::Display,
{
    let dot = to_dot(graph, config);
    std::fs::write(filename, dot).unwrap();
}

pub fn to_dot<N, E>(graph: &DiGraph<N, E>, config: &[petgraph::dot::Config]) -> String
where
    N: core::fmt::Display,
    E: core::fmt::Display,
{
    use petgraph::dot::Dot;

    let dot = format!(
        "{}",
        Dot::with_attr_getters(
            &graph,
            config,
            &|_, _| "bgcolor=\"#222222\"  fontcolor = \"#777777\" color = \"#777777\" ".to_string(),
            &|_, _| {
                "bgcolor=\"#222222\"  fontcolor = \"#cccccc\" color = \"#cccccc\" ".to_string()
            }
        )
    );
    dot.replace("digraph {", "digraph {\n    bgcolor=\"#131313\" ")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::traversal::{traverse, TraversalConfig, TraversalGraphingConfig};

    use super::*;
    use ::exhaustive::Exhaustive;
    use num_derive::{FromPrimitive, ToPrimitive};
    use num_traits::{FromPrimitive, ToPrimitive};
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
            let (_, graph, _) = traverse(
                CycleMachine.into(),
                Cycle::D,
                TraversalConfig::builder()
                    .graphing(TraversalGraphingConfig::default())
                    .build(),
                Some,
            )
            .unwrap();
            let graph = graph.unwrap();
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

    #[test]
    fn test_exhaustive_state_diagram() {
        use super::exhaustive::*;

        let config = DiagramConfig::default();

        let graph1 = state_diagram(CycleMachine, Cycle::D, &config);
        let nodes1: HashSet<_> = graph1.node_weights().collect();
        let edges1: HashSet<_> = graph1.edge_weights().collect();

        let graph2 = state_diagram(CycleMachine, Cycle::D, &config);
        let nodes2: HashSet<_> = graph2.node_weights().collect();
        let edges2: HashSet<_> = graph2.edge_weights().collect();

        assert_eq!(nodes1, nodes2);
        assert_eq!(edges1, edges2);
        println!("{}", to_dot(&graph1, &[]));
    }
}
