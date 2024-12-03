use exhaustive::Exhaustive;
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    hash::Hash,
    path::Path,
};

use crate::{util::first, Machine};

#[derive(Debug, Default, Clone)]
pub struct DiagramConfig {
    pub max_actions: Option<usize>,
    pub max_depth: Option<usize>,
    pub max_iters: Option<usize>,
    pub ignore_loopbacks: bool,
    pub trace_errors: bool,
}

// impl Default for DiagramConfig {
//     fn default() -> Self {
//         Self {
//             max_actions: None,
//             max_distance: None,
//             max_iters: None,
//             ignore_loopbacks: false,
//         }
//     }
// }

pub fn write_dot_state_diagram<M>(
    path: impl AsRef<Path>,
    machine: M,
    initial: M::State,
    config: &DiagramConfig,
) where
    M: Machine,
    M::State: Clone + Eq + Hash + Debug,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
    M::Error: Debug,
{
    write_dot_state_diagram_mapped(path, machine, initial, config, |m| Some(m), |a| Some(a))
}

pub fn write_dot_state_diagram_mapped<M, N, E>(
    path: impl AsRef<Path>,
    machine: M,
    initial: M::State,
    config: &DiagramConfig,
    map_node: impl Fn(M::State) -> Option<N>,
    map_edge: impl Fn(M::Action) -> Option<E>,
) where
    M: Machine,
    M::State: Clone + Eq + Hash,
    M::Action: Exhaustive + Clone + Eq + Hash,
    M::Error: Debug,
    N: Clone + Eq + Hash + Debug,
    E: Clone + Eq + Hash + Debug,
{
    use std::fs::File;
    use std::io::Write;
    let mut file = File::create(&path).unwrap();
    let graph = state_diagram_mapped(machine, initial, config, map_node, map_edge);
    let nodes = graph.node_count();
    let edges = graph.edge_count();
    write!(file, "{}", crate::diagram::to_dot(graph)).unwrap();
    println!(
        "wrote DOT diagram to '{}'. nodes={nodes}, edges={edges}",
        path.as_ref().display(),
    );
}

pub fn print_dot_state_diagram<M>(machine: M, initial: M::State, config: &DiagramConfig)
where
    M: Machine,
    M::State: Clone + Eq + Hash + Debug,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
    M::Error: Debug,
{
    print_dot_state_diagram_mapped::<M, M::State, M::Action>(
        machine,
        initial,
        config,
        |m| Some(m),
        |a| Some(a),
    )
}

pub fn print_dot_state_diagram_mapped<M, N, E>(
    machine: M,
    initial: M::State,
    config: &DiagramConfig,
    map_node: impl Fn(M::State) -> Option<N>,
    map_edge: impl Fn(M::Action) -> Option<E>,
) where
    M: Machine,
    M::State: Clone + Eq + Hash,
    M::Action: Exhaustive + Clone + Eq + Hash,
    M::Error: Debug,
    N: Clone + Eq + Hash + Debug,
    E: Clone + Eq + Hash + Debug,
{
    println!(
        "{}",
        crate::diagram::to_dot(state_diagram_mapped(
            machine, initial, config, map_node, map_edge
        ))
    );
}

pub fn state_diagram<M>(
    machine: M,
    initial: M::State,
    config: &DiagramConfig,
) -> DiGraph<M::State, M::Action>
where
    M: Machine,
    M::State: Clone + Eq + Hash + Debug,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
    M::Error: Debug,
{
    state_diagram_mapped(machine, initial, config, |m| Some(m), |a| Some(a))
}

/// Generate a state diagram of this state machine by exhaustively taking all possible actions
/// at each visited state.
pub fn state_diagram_mapped<M, N, E>(
    machine: M,
    initial: M::State,
    config: &DiagramConfig,
    map_node: impl Fn(M::State) -> Option<N>,
    map_edge: impl Fn(M::Action) -> Option<E>,
) -> DiGraph<N, E>
where
    M: Machine,
    M::State: Clone + Eq + Hash,
    M::Action: Exhaustive + Clone + Eq + Hash,
    M::Error: Debug,
    N: Clone + Eq + Hash + Debug,
    E: Clone + Eq + Hash + Debug,
{
    let mut graph = DiGraph::new();
    let mut visited_states: HashSet<M::State> = HashSet::new();
    let mut visited_nodes: HashMap<N, NodeIndex> = HashMap::new();
    let mut states_to_visit: VecDeque<(M::State, usize, Option<(M::Action, NodeIndex)>)> =
        VecDeque::new();
    let mut visited_edges = HashSet::new();

    states_to_visit.push_back((initial, 0, None));

    let mut total_steps = 0;
    let mut num_errors = 0;
    let mut num_terminations = 0;
    let mut num_iters = 0;

    while let Some((state, distance, origin)) = states_to_visit.pop_front() {
        num_iters += 1;
        if num_iters % 1000 == 0 {
            tracing::debug!("iter {num_iters}");
        }
        if config.max_iters.map(|m| num_iters >= m).unwrap_or(false) {
            panic!("max iters of {} reached", config.max_iters.unwrap());
        }
        let node: N = if let Some(node) = map_node(state.clone()) {
            node
        } else {
            continue;
        };
        let ix = if let Some(ix) = visited_nodes.get(&node) {
            *ix
        } else {
            tracing::debug!("new node (dist={distance}) : {node:?}");
            // Add the node to the graph.
            graph.add_node(node.clone())
        };

        // Add an edge from the previous node to this node.
        if let Some((prev_action, prev_ix)) = origin {
            if let Some(edge) = map_edge(prev_action) {
                if !(config.ignore_loopbacks && prev_ix == ix)
                    && visited_edges.insert((prev_ix, ix, edge.clone()))
                {
                    tracing::debug!("new edge : {edge:?}");
                    graph.add_edge(prev_ix, ix, edge);
                }
            }
        }

        // Don't explore the same node twice.
        if distance > config.max_depth.unwrap_or(usize::MAX) || visited_states.contains(&state) {
            tracing::debug!("skipping node (dist={distance}) : {node:?}");
            continue;
        }

        visited_states.insert(state.clone());
        visited_nodes.insert(node.clone(), ix);

        // If this is a terminal state, no need to explore further.
        if machine.is_terminal(&state) {
            num_terminations += 1;
            continue;
        }

        // Queue up visits to all nodes reachable from this node..
        for edge in M::Action::iter_exhaustive(config.max_actions) {
            total_steps += 1;
            match machine.transition(state.clone(), edge.clone()).map(first) {
                Ok(node) => {
                    states_to_visit.push_back((node, distance + 1, Some((edge, ix))));
                }
                Err(err) => {
                    if config.trace_errors {
                        tracing::error!("error: {err:?}");
                    }
                    num_errors += 1;
                }
            }
        }
    }

    tracing::info!(
        "constructed state diagram in {total_steps} total steps ({num_errors} errors, {num_terminations} terminations). nodes={}, edges={}",
        graph.node_count(),
        graph.edge_count(),
    );

    graph
}
