use exhaustive::Exhaustive;
use petgraph::graph::{DiGraph, NodeIndex};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    hash::Hash,
};

use crate::{util::first, Machine};

#[derive(Debug, Clone)]
pub struct DiagramConfig {
    pub max_actions: Option<usize>,
    pub max_distance: Option<usize>,
    pub ignore_loopbacks: bool,
}

impl Default for DiagramConfig {
    fn default() -> Self {
        Self {
            max_actions: None,
            max_distance: None,
            ignore_loopbacks: false,
        }
    }
}

pub fn print_dot_state_diagram<M>(m: M, config: &DiagramConfig)
where
    M: Machine + Clone + Eq + Debug + Hash,
    M::Action: Exhaustive + Clone + Eq + Debug + Hash,
{
    println!("{}", crate::diagram::to_dot(state_diagram(m, config)));
}

/// Generate a state diagram of this state machine by exhaustively taking all possible actions
/// at each visited state.
pub fn state_diagram<M>(m: M, config: &DiagramConfig) -> DiGraph<M, M::Action>
where
    M: Machine + Clone + Eq + Hash + Debug,
    M::Action: Exhaustive + Clone + Eq + Hash,
{
    let mut graph = DiGraph::new();
    let mut visited_nodes = HashMap::new();
    let mut nodes_to_visit: VecDeque<(M, usize, Option<(M::Action, NodeIndex)>)> = VecDeque::new();
    let mut edges = HashSet::new();

    nodes_to_visit.push_back((m, 0, None));

    let mut total_steps = 0;
    let mut num_errors = 0;
    let mut num_terminations = 0;

    while let Some((node, distance, origin)) = nodes_to_visit.pop_front() {
        let ix = if let Some(ix) = visited_nodes.get(&node) {
            *ix
        } else {
            // Add the node to the graph.
            graph.add_node(node.clone())
        };

        // Add an edge from the previous node to this node.
        if let Some((prev_edge, prev_ix)) = origin {
            if !(config.ignore_loopbacks && prev_ix == ix)
                && edges.insert((prev_ix, ix, prev_edge.clone()))
            {
                graph.add_edge(prev_ix, ix, prev_edge);
            }
        }

        // If this is a terminal state, no need to explore further.
        if node.is_terminal() {
            num_terminations += 1;
            continue;
        }

        // Don't explore the same node twice.
        if distance > config.max_distance.unwrap_or(usize::MAX) && visited_nodes.contains_key(&node)
        {
            continue;
        }

        // Queue up visits to all nodes reachable from this node..
        for edge in M::Action::iter_exhaustive(config.max_actions) {
            total_steps += 1;
            match node.clone().transition(edge.clone()).map(first) {
                Ok(node) => {
                    nodes_to_visit.push_back((node, distance + 1, Some((edge, ix))));
                }
                Err(_err) => {
                    num_errors += 1;
                }
            }
        }

        visited_nodes.insert(node.clone(), ix);
    }

    tracing::info!(
        "constructed state diagram in {total_steps} total steps ({num_errors} errors, {num_terminations} terminations). nodes={}, edges={}",
        graph.node_count(),
        graph.edge_count(),
    );

    graph
}
