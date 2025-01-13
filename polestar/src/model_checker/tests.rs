use crate::logic::{conjoin, Propositions};

use super::*;

const MODULO: usize = 16;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct TestMachine1;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct TestMachine2;

const LOOP: u8 = 4;

impl Machine for TestMachine1 {
    type State = u8;
    type Action = ();
    type Error = anyhow::Error;
    type Fx = ();

    fn transition(&self, state: Self::State, (): Self::Action) -> TransitionResult<Self> {
        Ok((state.wrapping_add(2) % MODULO as u8, ()))
    }

    fn is_terminal(&self, _: &Self::State) -> bool {
        false
    }
}

impl Machine for TestMachine2 {
    type State = u8;
    type Action = bool;
    type Error = anyhow::Error;
    type Fx = ();

    fn transition(&self, state: Self::State, bump: Self::Action) -> TransitionResult<Self> {
        let n = LOOP;
        let next = if state < n {
            if bump && state != n - 1 {
                state + 1
            } else {
                state * n
            }
        } else {
            let group = state / n;
            (group * n) + ((state + 1) % n)
        };
        Ok((next, ()))
    }

    fn is_terminal(&self, _: &Self::State) -> bool {
        false
    }
}

impl Propositions<String> for Transition<TestMachine2> {
    fn eval(&self, p: &String) -> bool {
        let Transition(s, _, ss) = *self;
        match p.as_str() {
            "even" => s % 2 == 0,
            "max" => s == (MODULO - 1) as u8,
            "building" => s < 4,
            "loopmin" => s % LOOP == 0,
            "increasing" => s < ss,
            "is1" => s == 1,
            "is2" => s == 2,
            "is3" => s == 3,
            "is4" => s == 4,
            "is5" => s == 5,
            "is6" => s == 6,
            "is7" => s == 7,
            "is8" => s == 8,
            "is9" => s == 9,
            "is11" => s == 11,
            "is15" => s == 15,
            p => unreachable!("can't eval unknown prop '{p}' with state {s}"),
        }
    }
}

#[allow(unused)]
#[derive(Debug, derive_more::Display, Clone, Copy, PartialEq, Eq)]
#[display("({}, {})", _0, _1)]
struct Node(u8, bool);

//  ███████████ ██████████  █████████  ███████████
// ░█░░░███░░░█░░███░░░░░█ ███░░░░░███░█░░░███░░░█
// ░   ░███  ░  ░███  █ ░ ░███    ░░░ ░   ░███  ░
//     ░███     ░██████   ░░█████████     ░███
//     ░███     ░███░░█    ░░░░░░░░███    ░███
//     ░███     ░███ ░   █ ███    ░███    ░███
//     █████    ██████████░░█████████     █████
//    ░░░░░    ░░░░░░░░░░  ░░░░░░░░░     ░░░░░

#[test]
fn model_checker_test() {
    // true negatives:
    let negatives = conjoin([
        "G ( is4 -> ! X is8 ) ",
        "G ( is4 -> ! F is2 )",
        "G ( !is1 || ( (F is4 && !(F is2)) || (F is2 && !(F is4)) ))",
        "G ( is3 -> ! G F is5 )",
    ]);

    // true positives:
    let positives = conjoin([
        "G ( (is2 && X is8) -> G F is11)",
        "G ( is1 -> (G F is5 || G F is3 || G F is8) )",
        "G ( increasing || X loopmin )",
    ]);

    let ltl = conjoin([
        positives,
        negatives,
    ]);

    println!("ltl: {ltl}");

    TestMachine2
        .traverse([1])
        .specced((), &ltl)
        .unwrap()
        .model_check_report()
        .unwrap();

    // write_dot_state_diagram_mapped(
    //     "promela-diagram.dot",
    //     machine.clone(),
    //     initial.clone(),
    //     &DiagramConfig {
    //         max_depth: None,
    //         ..Default::default()
    //     },
    //     |s| Some(format!("{s:?}")),
    //     Some,
    // );

    // let config = TraversalConfig::builder()
    //     .record_terminals(false)
    //     .trace_every(1000)
    //     .graphing(TraversalGraphingConfig::default())
    //     .is_fatal_error(|e| !matches!(e, ModelCheckerTransitionError::MachineError(_)))
    //     .visitor(|s: &ModelCheckerState<u8, bool>, _| {
    //         // println!(
    //         //     "<:> {}: buchi {:?} path {:?}",
    //         //     &s.state.state, &s.buchi, s.state.path
    //         // );
    //         Ok(())
    //     })
    //     .build();

    // let (report, graph, _) = traverse(checker, initial, config, Some).unwrap();

    // let graph = graph.unwrap();

    // {
    //     let graph = graph.map(
    //         |_, n: &ModelCheckerState<u8, bool>| Node(n.state, n.buchi.is_accepting()),
    //         |_, e| e,
    //     );
    //     crate::diagram::write_dot(
    //         "promela-verify.dot",
    //         &graph,
    //         // &[petgraph::dot::Config::EdgeNoLabel],
    //         &[],
    //     );
    // }

    // dbg!(&report);

    // let condensed = petgraph::algo::condensation(graph, true);

    // let leaves = condensed.node_indices().filter(|n| {
    //     let outgoing = condensed
    //         .neighbors_directed(*n, petgraph::Direction::Outgoing)
    //         .count();
    //     outgoing == 0
    // });

    // {
    //     let condensed = condensed.map(
    //         |_, n| {
    //             n.iter()
    //                 .map(|s| {
    //                     let tup = (s.pathstate.state, &s.buchi);
    //                     format!("{tup:?}")
    //                 })
    //                 .collect_vec()
    //                 .join("\n")
    //         },
    //         |_, e| e,
    //     );

    //     crate::diagram::write_dot(
    //         "promela-verify-condensed.dot",
    //         &condensed,
    //         &[petgraph::dot::Config::EdgeNoLabel],
    //         // &[],
    //     );
    // }

    // for index in leaves {
    //     let scc = condensed.node_weight(index).unwrap();
    //     let accepting = scc.iter().any(|n| n.buchi.is_accepting());
    //     if !accepting {
    //         let mut paths = scc.iter().map(|n| n.pathstate.path.clone()).collect_vec();
    //         paths.sort();
    //         dbg!(&paths);
    //         panic!("non-accepting SCC found");
    //     }
    // }
}

#[test]
#[ignore = "diagram"]
fn model_checker_diagram() {
    let graph = TestMachine2.traverse([1]).graph().unwrap();

    crate::diagram::write_dot(
        "out.dot",
        &graph,
        // &[petgraph::dot::Config::EdgeNoLabel],
        &[],
    );
}
