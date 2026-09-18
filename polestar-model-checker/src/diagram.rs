//! Functions for generating DOT representations of state graphs.

use petgraph::graph::DiGraph;

/// Write a DOT representation of a graph to a file
pub fn write_dot<N, E>(filename: &str, graph: &DiGraph<N, E>, config: &[petgraph::dot::Config])
where
    N: core::fmt::Display,
    E: core::fmt::Display,
{
    let dot = to_dot(graph, config);
    std::fs::write(filename, dot).unwrap();
}

/// Get a DOT representation of a graph
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
