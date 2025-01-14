use std::{
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    marker::PhantomData,
    process::Command,
    sync::Arc,
};

use anyhow::anyhow;

use crate::{
    logic::{
        EvaluatePropositions, LogicStatement, PropositionBindings, PropositionMapping, Transition,
    },
    Machine, TransitionResult,
};

#[derive(derive_more::Debug)]
pub(crate) struct BuchiAutomaton<M: Machine, PM: PropositionMapping> {
    pub states: HashMap<StateName, Arc<BuchiState>>,

    #[debug(skip)]
    propmap: PM,
    #[debug(skip)]
    phantom: PhantomData<M>,
}

impl<M, PM> Machine for BuchiAutomaton<M, PM>
where
    M: Machine,
    PM: PropositionMapping + Send + Sync + 'static,
    Transition<M>: EvaluatePropositions<PM::Proposition>,
{
    type State = BuchiPaths;
    type Action = Transition<M>;
    type Error = BuchiError;
    type Fx = ();

    fn transition(
        &self,
        state: Self::State,
        inner_transition: Self::Action,
    ) -> TransitionResult<Self> {
        let props = PropositionBindings {
            props: &self.propmap,
            transition: inner_transition,
        };
        let next = state
            .0
            .into_iter()
            .flat_map(|buchi_name| {
                let buchi_state = self
                    .states
                    .get(&buchi_name)
                    .expect("no buchi state named '{buchi_name}'. This is a polestar bug.");
                // .ok_or_else(|| {
                //     BuchiError::Internal(anyhow!(
                //         "no buchi state named '{buchi_name}'. This is a polestar bug."
                //     ))
                // })?;
                match &**buchi_state {
                    BuchiState::Skip => BTreeSet::new(), // vec![Ok((buchi_name, buchi_state))],
                    BuchiState::Conditional { predicates, .. } => predicates
                        .iter()
                        .filter_map(|(ltl, name)| ltl.eval(&props).then_some(name))
                        .cloned()
                        .collect::<BTreeSet<_>>(),
                }
            })
            .collect::<BTreeSet<_>>();

        if next.is_empty() {
            return Err(BuchiError::LtlError(anyhow!("LTL not satisfied")));
        }

        Ok((next.into(), ()))
    }

    fn is_terminal(&self, _: &Self::State) -> bool {
        false
    }
}

/// Errors while transitioning a Buchi automaton.
#[derive(Debug)]
pub enum BuchiError {
    /// The LTL expression is not satisfied.
    LtlError(anyhow::Error),
    // Internal(anyhow::Error),
}

impl<M: Machine, PM: PropositionMapping> BuchiAutomaton<M, PM> {
    pub fn from_ltl(propmap: PM, ltl_str: &str) -> Result<Self, anyhow::Error> {
        let output = Command::new("ltl3ba")
            .args(["-f", ltl_str])
            .output()
            .unwrap();

        let promela = String::from_utf8_lossy(&output.stdout);

        if promela.contains("expected predicate, saw") {
            return Err(anyhow!("ltl3ba couldn't parse LTL input. Error: {promela}"));
        }

        Ok(Self::from_promela(propmap, &promela))
    }

    pub fn from_promela(propmap: PM, promela: &str) -> Self {
        let lines = promela.lines().collect::<Vec<_>>();

        let pat_state = regex::Regex::new("^(\\w+):").unwrap();
        let pat_transition = regex::Regex::new(":: (.+) -> goto (.+)").unwrap();

        let mut states = HashMap::new();

        let mut current: Option<(StateName, BuchiState)> = None;

        for line in lines {
            if let Some(captures) = pat_state.captures(line) {
                if let Some((name, state)) = current {
                    states.insert(name.to_string(), Arc::new(state));
                }
                let name = captures.get(1).unwrap().as_str().to_string();
                let accepting = name.contains("accept_");
                current = Some((
                    name,
                    BuchiState::Conditional {
                        accepting,
                        predicates: vec![],
                    },
                ));
            } else if let Some(captures) = pat_transition.captures(line) {
                let predicate = captures.get(1).unwrap().as_str();
                let next = captures.get(2).unwrap().as_str();
                if let BuchiState::Conditional { predicates, .. } = &mut current.as_mut().unwrap().1
                {
                    predicates.push((
                        LogicStatement::from_promela_predicate(predicate).unwrap(),
                        next.to_string(),
                    ));
                } else {
                    unreachable!()
                }
            } else if line.contains("skip") {
                current.as_mut().unwrap().1 = BuchiState::Skip
            }
        }

        let (state_name, state) = current.unwrap();
        states.insert(state_name, Arc::new(state));

        Self {
            states,
            propmap,
            phantom: PhantomData,
        }
    }
}

pub(crate) type StateName = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Deref, derive_more::From)]
pub(crate) struct BuchiPaths(pub(crate) BTreeSet<StateName>);

impl BuchiPaths {
    pub fn is_accepting(&self) -> bool {
        self.iter().any(|n| n.starts_with("accept_"))
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum BuchiState {
    Conditional {
        accepting: bool,
        predicates: Vec<(LogicStatement, StateName)>,
    },
    Skip,
}

impl BuchiState {
    /// Is this an accepting state?
    ///
    /// A path through a Buchi automaton is accepted if the path always eventually
    /// passes through an accepting state.
    /// (see https://en.wikipedia.org/wiki/B%C3%BCchi_automaton)
    pub fn is_accepting(&self) -> bool {
        match self {
            BuchiState::Skip => true,
            BuchiState::Conditional { accepting, .. } => *accepting,
        }
    }
}

impl Debug for BuchiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuchiState::Conditional {
                accepting,
                predicates,
            } => {
                {
                    let mut list = f.debug_list();
                    for (ltl, next) in predicates.iter() {
                        list.entry(&format_args!("{ltl:?} -> {next}"));
                    }
                    list.finish()?
                };
                f.debug_struct("Conditional")
                    .field("accepting", accepting)
                    .field("predicates", &())
                    .finish()
            }
            BuchiState::Skip => write!(f, "AllAccepting"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::logic::PropositionRegistry;

    use super::*;

    #[test]
    fn from_promela_never() {
        let promela = r#"
never { /* G( ( call && F open) -> ((!at_floor && !open) U (open || ((at_floor && !open) U (open || ((!at_floor && !open) U (open || ((at_floor && !open) U (open || (!at_floor U open)))))))))) */
accept_init:
	if
	:: (!call) || (call && open) -> goto accept_init
	:: (call && !open && at_floor) -> goto T3_S4
	:: (call && !open && !at_floor) -> goto T4_S9
	:: (call && !open) -> goto accept_S10
	fi;
T0_S1:
	if
	:: (open) -> goto accept_init
	:: (!open && !at_floor) -> goto T0_S1
	fi;
T1_S2:
	if
	:: (open) -> goto accept_init
	:: (!open && at_floor) -> goto T1_S2
	:: (!open && !at_floor) -> goto accept_S5
	fi;
T2_S3:
	if
	:: (open) -> goto accept_init
	:: (!open && !at_floor) -> goto T2_S3
	:: (!open && !at_floor) -> goto accept_S5
	:: (!open && at_floor) -> goto accept_S6
	fi;
T3_S4:
	if
	:: (open) -> goto accept_init
	:: (!open && at_floor) -> goto T3_S4
	:: (!open && at_floor) -> goto accept_S6
	:: (!open && !at_floor) -> goto accept_S7
	fi;
accept_S5:
	if
	:: (open) -> goto accept_init
	:: (!open && !at_floor) -> goto T0_S1
	fi;
accept_S6:
	if
	:: (open) -> goto accept_init
	:: (!open && !at_floor) -> goto T0_S1
	:: (!open && at_floor) -> goto T1_S2
	fi;
accept_S7:
	if
	:: (open) -> goto accept_init
	:: (!open && at_floor) -> goto T1_S2
	:: (!open && !at_floor) -> goto T2_S3
	fi;
accept_S8:
	if
	:: (open) -> goto accept_init
	:: (!open && !at_floor) -> goto T2_S3
	:: (!open && at_floor) -> goto T3_S4
	fi;
T4_S9:
	if
	:: (open) -> goto accept_init
	:: (!open && !at_floor) -> goto accept_S7
	:: (!open && at_floor) -> goto accept_S8
	:: (!open && !at_floor) -> goto T4_S9
	fi;
accept_S10:
	if
	:: (!open) -> goto accept_S10
	fi;
}
        "#;
        let propmap = PropositionRegistry::<String>::new(["open", "call", "at_floor"]).unwrap();
        let machine =
            BuchiAutomaton::<(), PropositionRegistry<String>>::from_promela(propmap, promela);
        dbg!(&machine);
    }
}
