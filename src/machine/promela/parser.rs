use std::{collections::HashMap, process::Command, sync::Arc};

use nom::{
    branch::alt, bytes::complete::tag, character::complete::*, combinator::map_res,
    error::ErrorKind, multi::*, sequence::*, AsChar, IResult, InputTakeAtPosition,
};

pub struct PromelaBuchi {
    pub states: HashMap<StateName, Arc<BuchiState>>,
}

impl PromelaBuchi {
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

        let mut state = BuchiState::default();
        let mut state_name: Option<String> = None;

        for line in lines {
            if let Some(captures) = pat_state.captures(line) {
                if let Some(state_name) = state_name {
                    states.insert(state_name.to_string(), Arc::new(state));
                }
                state_name = Some(captures.get(1).unwrap().as_str().to_string());
                state = BuchiState::default();
            } else if let Some(captures) = pat_transition.captures(line) {
                let ltl = captures.get(1).unwrap().as_str();
                let next = captures.get(2).unwrap().as_str();
                state.0.push((parse_ltl(&ltl).unwrap(), next.to_string()));
            }
        }

        states.insert(state_name.unwrap(), Arc::new(state));

        dbg!(&states);
        Self { states }
    }
}

pub type StateName = String;

#[derive(Debug, Default, PartialEq, Eq, Hash, derive_more::Deref)]
pub struct BuchiState(Vec<(Ltl, StateName)>);

fn parse_prop(input: &str) -> IResult<&str, Ltl> {
    let (rest, s) = input.split_at_position1_complete(
        |item| !item.is_alphanum() && !['_'].contains(&item),
        ErrorKind::AlphaNumeric,
    )?;

    Ok((rest, Ltl::Prop(s.to_string())))

    // map_res(alpha1, |s: &str| {
    //     Result::<_, nom::error::Error<&str>>::Ok(Ltl::Prop(s.to_string()))
    // })(input)
}

fn parse_neg(input: &str) -> IResult<&str, Ltl> {
    map_res(preceded(char('!'), parse_prop), |s| {
        Result::<_, nom::error::Error<&str>>::Ok(Ltl::Not(Box::new(s)))
    })(input)
}

fn parse_conj(input: &str) -> IResult<&str, Ltl> {
    let (rest, vs) = separated_list1(tag(" && "), alt((parse_neg, parse_prop)))(input)?;
    Ok((
        rest,
        vs.into_iter()
            .reduce(|a, b| Ltl::And(Box::new(a), Box::new(b)))
            .unwrap(),
    ))
}

fn parse_parens(input: &str) -> IResult<&str, Ltl> {
    delimited(char('('), parse_conj, char(')'))(input)
}

fn parse_disj(input: &str) -> IResult<&str, Ltl, nom::error::Error<&str>> {
    let (rest, vs) = separated_list1(tag(" || "), parse_parens)(input)?;
    Ok((
        rest,
        vs.into_iter()
            .reduce(|a, b| Ltl::Or(Box::new(a), Box::new(b)))
            .unwrap(),
    ))
}

pub fn parse_ltl(input: &str) -> Result<Ltl, nom::error::Error<&str>> {
    let (rest, expr) = parse_disj(input).unwrap();
    assert!(rest.is_empty());
    Ok(expr)
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Ltl {
    Prop(String),
    Not(Box<Ltl>),
    And(Box<Ltl>, Box<Ltl>),
    Or(Box<Ltl>, Box<Ltl>),
}

impl Ltl {
    pub fn eval(&self, props: &impl Propositions) -> bool {
        match self {
            Ltl::Prop(s) => props.eval(s),
            Ltl::Not(e) => !e.eval(props),
            Ltl::And(a, b) => a.eval(props) && b.eval(props),
            Ltl::Or(a, b) => a.eval(props) || b.eval(props),
        }
    }
}

pub trait Propositions {
    fn eval(&self, prop: &str) -> bool;
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
        let machine = PromelaBuchi::from_promela(promela);
        // dbg!(&machine);
    }
}
