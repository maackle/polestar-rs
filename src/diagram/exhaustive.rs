use exhaustive::Exhaustive;
use petgraph::graph::{DiGraph, NodeIndex};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    hash::Hash,
    path::Path,
};

use crate::{util::first, Machine};

#[derive(Debug, Clone)]
pub struct DiagramConfig {
    pub max_actions: Option<usize>,
    pub max_distance: Option<usize>,
    pub max_iters: Option<usize>,
    pub ignore_loopbacks: bool,
}

impl Default for DiagramConfig {
    fn default() -> Self {
        Self {
            max_actions: None,
            max_distance: None,
            max_iters: None,
            ignore_loopbacks: false,
        }
    }
}

pub fn write_dot_state_diagram<M>(
    path: impl AsRef<Path>,
    machine: M,
    initial: M::State,
    config: &DiagramConfig,
) where
    M: Machine,
    M::State: Clone + Eq + Hash + Debug,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
{
    use std::fs::File;
    use std::io::Write;
    let mut file = File::create(&path).unwrap();
    write!(
        file,
        "{}",
        crate::diagram::to_dot(state_diagram(machine, initial, config))
    )
    .unwrap();
    println!("wrote DOT diagram to {}", path.as_ref().display());
}

pub fn print_dot_state_diagram<M>(machine: M, initial: M::State, config: &DiagramConfig)
where
    M: Machine,
    M::State: Clone + Eq + Hash + Debug,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
{
    print_dot_state_diagram_mapped::<M, M::State>(machine, initial, config, |m| m)
}

pub fn print_dot_state_diagram_mapped<M, N>(
    machine: M,
    initial: M::State,
    config: &DiagramConfig,
    map: impl Fn(M::State) -> N,
) where
    M: Machine,
    M::State: Clone + Eq + Hash + Debug,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
    N: Clone + Eq + Hash + Debug,
{
    println!(
        "{}",
        crate::diagram::to_dot(state_diagram_mapped(machine, initial, config, map))
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
{
    state_diagram_mapped(machine, initial, config, |m| m)
}

/// Generate a state diagram of this state machine by exhaustively taking all possible actions
/// at each visited state.
pub fn state_diagram_mapped<M, N>(
    mut machine: M,
    initial: M::State,
    config: &DiagramConfig,
    map_node: impl Fn(M::State) -> N,
) -> DiGraph<N, M::Action>
where
    M: Machine,
    M::State: Clone + Eq + Hash + Debug,
    M::Action: Exhaustive + Clone + Eq + Hash + Debug,
    N: Clone + Eq + Hash + Debug,
{
    dbg!();
    let mut graph = DiGraph::new();
    let mut visited_states: HashSet<M::State> = HashSet::new();
    let mut visited_nodes: HashMap<N, NodeIndex> = HashMap::new();
    let mut states_to_visit: VecDeque<(M::State, usize, Option<(M::Action, NodeIndex)>)> =
        VecDeque::new();
    let mut edges = HashSet::new();

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
        let node: N = map_node(state.clone());
        let ix = if let Some(ix) = visited_nodes.get(&node) {
            *ix
        } else {
            tracing::debug!("new node (dist={distance}) : {node:?}");
            // Add the node to the graph.
            graph.add_node(node.clone())
        };

        // Add an edge from the previous node to this node.
        if let Some((prev_edge, prev_ix)) = origin {
            if !(config.ignore_loopbacks && prev_ix == ix)
                && edges.insert((prev_ix, ix, prev_edge.clone()))
            {
                tracing::debug!("new edge : {prev_edge:?}");
                graph.add_edge(prev_ix, ix, prev_edge);
            }
        }

        // Don't explore the same node twice.
        if distance > config.max_distance.unwrap_or(usize::MAX) || visited_states.contains(&state) {
            tracing::debug!("skipping node (dist={distance}) : {state:?}");
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
                Err(_err) => {
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
