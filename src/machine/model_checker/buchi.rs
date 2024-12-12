use std::{
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    marker::PhantomData,
    process::Command,
    sync::Arc,
};

use anyhow::anyhow;

use crate::{
    logic::{LogicPredicate, Propositions},
    Machine, TransitionResult,
};

#[derive(Debug)]
pub struct BuchiAutomaton<P> {
    pub states: HashMap<StateName, Arc<BuchiState>>,
    phantom: PhantomData<P>,
}

impl<P> Machine for BuchiAutomaton<P>
where
    P: Propositions,
{
    type State = BuchiPaths;
    type Action = P;
    type Error = BuchiError;
    type Fx = ();

    fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self> {
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
                    BuchiState::AcceptAll => todo!(), // vec![Ok((buchi_name, buchi_state))],
                    BuchiState::Conditional { predicates, .. } => {
                        (predicates.len());
                        predicates
                            .iter()
                            .filter_map(|(ltl, name)| ((ltl).eval(&action)).then_some(name))
                            .cloned()
                            .collect::<BTreeSet<_>>()
                    }
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

#[derive(Debug)]
pub enum BuchiError {
    Internal(anyhow::Error),
    LtlError(anyhow::Error),
}

impl<P> BuchiAutomaton<P> {
    pub fn from_ltl(ltl_str: &str) -> Self {
        let promela = Command::new("ltl3ba")
            .args(["-f", &format!("{ltl_str}")])
            .output()
            .unwrap();
        Self::from_promela(String::from_utf8_lossy(&promela.stdout).as_ref())
    }

    pub fn from_promela(promela: &str) -> Self {
        println!("{}", promela);
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
                        LogicPredicate::from_promela_predicate(&predicate).unwrap(),
                        next.to_string(),
                    ));
                } else {
                    unreachable!()
                }
            } else if line.contains("skip") {
                current.as_mut().unwrap().1 = BuchiState::AcceptAll
            }
        }

        let (state_name, state) = current.unwrap();
        states.insert(state_name, Arc::new(state));

        Self {
            states,
            phantom: PhantomData,
        }
    }
}

pub type StateName = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Deref, derive_more::From)]
pub struct BuchiPaths(pub(crate) BTreeSet<StateName>);

impl BuchiPaths {
    pub fn is_accepting(&self) -> bool {
        self.iter().any(|n| n.starts_with("accept_"))
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum BuchiState {
    Conditional {
        accepting: bool,
        predicates: Vec<(LogicPredicate, StateName)>,
    },
    AcceptAll,
}

impl BuchiState {
    pub fn is_accepting(&self) -> bool {
        match self {
            BuchiState::AcceptAll => true,
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
                let predicates = {
                    let mut list = f.debug_list();
                    for (ltl, next) in predicates.iter() {
                        list.entry(&format_args!("{ltl:?} -> {next}"));
                    }
                    list.finish()?
                };
                f.debug_struct("Conditional")
                    .field("accepting", accepting)
                    .field("predicates", &predicates)
                    .finish()
            }
            BuchiState::AcceptAll => write!(f, "AllAccepting"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_promela_never() {
        let promela = r#"
never { /* G( ( call && F open) -> ((!at-floor && !open) U (open || ((at-floor && !open) U (open || ((!at-floor && !open) U (open || ((at-floor && !open) U (open || (!at-floor U open)))))))))) */
accept_init:
	if
	:: (!call) || (call && open) -> goto accept_init
	:: (call && !open && at-floor) -> goto T3_S4
	:: (call && !open && !at-floor) -> goto T4_S9
	:: (call && !open) -> goto accept_S10
	fi;
T0_S1:
	if
	:: (open) -> goto accept_init
	:: (!open && !at-floor) -> goto T0_S1
	fi;
T1_S2:
	if
	:: (open) -> goto accept_init
	:: (!open && at-floor) -> goto T1_S2
	:: (!open && !at-floor) -> goto accept_S5
	fi;
T2_S3:
	if
	:: (open) -> goto accept_init
	:: (!open && !at-floor) -> goto T2_S3
	:: (!open && !at-floor) -> goto accept_S5
	:: (!open && at-floor) -> goto accept_S6
	fi;
T3_S4:
	if
	:: (open) -> goto accept_init
	:: (!open && at-floor) -> goto T3_S4
	:: (!open && at-floor) -> goto accept_S6
	:: (!open && !at-floor) -> goto accept_S7
	fi;
accept_S5:
	if
	:: (open) -> goto accept_init
	:: (!open && !at-floor) -> goto T0_S1
	fi;
accept_S6:
	if
	:: (open) -> goto accept_init
	:: (!open && !at-floor) -> goto T0_S1
	:: (!open && at-floor) -> goto T1_S2
	fi;
accept_S7:
	if
	:: (open) -> goto accept_init
	:: (!open && at-floor) -> goto T1_S2
	:: (!open && !at-floor) -> goto T2_S3
	fi;
accept_S8:
	if
	:: (open) -> goto accept_init
	:: (!open && !at-floor) -> goto T2_S3
	:: (!open && at-floor) -> goto T3_S4
	fi;
T4_S9:
	if
	:: (open) -> goto accept_init
	:: (!open && !at-floor) -> goto accept_S7
	:: (!open && at-floor) -> goto accept_S8
	:: (!open && !at-floor) -> goto T4_S9
	fi;
accept_S10:
	if
	:: (!open) -> goto accept_S10
	fi;
}

        "#;
        let machine = BuchiAutomaton::<()>::from_promela(promela);
        dbg!(&machine);
    }
}
