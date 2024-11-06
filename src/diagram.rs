use petgraph::graph::DiGraph;

mod is_terminal;
pub use is_terminal::*;

pub mod arbitrary;
pub mod exhaustive;

pub use arbitrary::*;

pub fn to_dot<N, E>(graph: DiGraph<N, E>) -> String
where
    N: core::fmt::Debug,
    E: core::fmt::Debug,
{
    use petgraph::dot::Dot;
    format!("{:?}", Dot::with_config(&graph, &[]))
}
