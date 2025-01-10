use crate::{
    logic::Propositions,
    traversal::{traverse, TraversalConfig, TraversalGraphingConfig},
};
use itertools::Itertools;

use super::*;
use crate::diagram::exhaustive::*;

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
    let negatives = [
        "G ( is4 -> X is5 ) ",
        "G ( is4 -> X is8 ) ",
        "G ( is4 -> F is2 )",
        "G ( (F is4 && !(F is2)) || (F is2 && !(F is4)) )",
        "G ( is3 -> G F is5 )",
        "G ( is2 -> F is4 )",
    ];

    // true positives:
    let positives = [
        "G ( (is2 && X is6) -> G F is5)",
        "G ( is1 -> (G F is5 || G F is3 || G F is8) )",
        "G ( increasing || X loopmin )",
    ];

    let ltl = format!(
        "({}) && ({})",
        positives.map(|p| format!("({p})")).join(" && "),
        negatives.map(|p| format!("!({p})")).join(" && "),
    );

    let checker = ModelChecker::new(TestMachine2, (), &ltl).unwrap();
    let initial = checker.initial(1);

    model_checker_report(checker.check(1));

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
#[cfg(todo)]
fn test_checker() {
    tracing_subscriber::fmt::init();

    let even = P::atom("is-even".to_string(), |s: &u8| s % 2 == 0);
    let small = P::atom("single-digit".to_string(), |s: &u8| *s < 10);
    let big = P::atom("20-and-up".to_string(), |s: &u8| *s >= 20);
    let reallybig = P::atom("100-and-up".to_string(), |s: &u8| *s >= 100);
    let not_teens = small.clone().or(big.clone());

    let redundant = P::or(reallybig.clone(), reallybig.clone());

    let checker = Mach.checked().with_predicates([
        P::always(even.clone().implies(P::next(P::not(even.clone())))),
        P::always(P::not(even.clone()).implies(P::next(even.clone()))),
        P::always(not_teens),
        P::eventually(reallybig),
    ]);

    checker.check_fold(0, [1, 2, 3, 108, 21]).unwrap();

    let err = checker.check_fold(0, [1, 2, 3, 23, 21]).unwrap_err();
    dbg!(&err);
    assert_eq!(err.unwrap_predicate().path, vector![1, 2, 3, 23, 21]);

    let err = checker.check_fold(1, [2, 12, 33]).unwrap_err();
    dbg!(&err);
    assert_eq!(err.unwrap_predicate().path, vector![2, 12]);

    Mach.checked()
        .with_predicates([P::always(redundant)])
        .check_fold(0, [100])
        .unwrap();
}

#[test]
#[ignore = "diagram"]
fn model_checker_diagram() {
    let (_, graph, _) = traverse(
        TestMachine2.into(),
        1,
        TraversalConfig {
            // record_terminals: true,
            // trace_every: 1,
            graphing: Some(TraversalGraphingConfig::default()),
            ..Default::default()
        },
        // .with_fatal_error(|e| !matches!(e, BuchiError::MachineError(_))),
        Some,
    )
    .unwrap();

    let graph = graph.unwrap();

    crate::diagram::write_dot(
        "out.dot",
        &graph,
        // &[petgraph::dot::Config::EdgeNoLabel],
        &[],
    );

    write_dot_state_diagram_mapped(
        "promela-diagram.dot",
        TestMachine2,
        1,
        &DiagramConfig {
            max_depth: None,
            ..Default::default()
        },
        Some,
        Some,
    );
}
