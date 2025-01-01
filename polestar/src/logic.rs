use std::{collections::HashMap, fmt::Display};

use itertools::Itertools;

use crate::Machine;

mod promela_parser;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogicPredicate {
    True,
    False,

    Prop(String),

    And(BoxPredicate, BoxPredicate),
    Or(BoxPredicate, BoxPredicate),
    Not(BoxPredicate),
    Implies(BoxPredicate, BoxPredicate),
}

type BoxPredicate = Box<LogicPredicate>;

impl LogicPredicate {
    pub fn eval(&self, props: &impl Propositions<String>) -> bool {
        match self {
            LogicPredicate::True => true,
            LogicPredicate::False => false,

            LogicPredicate::Prop(name) => props.eval(name),
            LogicPredicate::And(p1, p2) => p1.eval(props) && p2.eval(props),
            LogicPredicate::Or(p1, p2) => p1.eval(props) || p2.eval(props),
            LogicPredicate::Not(p) => !p.eval(props),
            LogicPredicate::Implies(p1, p2) => !p1.eval(props) || p2.eval(props),
        }
    }

    pub fn not(self) -> Self {
        LogicPredicate::Not(Box::new(self))
    }

    pub fn and(self, p2: Self) -> Self {
        LogicPredicate::And(Box::new(self), Box::new(p2))
    }

    pub fn or(self, p2: Self) -> Self {
        LogicPredicate::Or(Box::new(self), Box::new(p2))
    }

    pub fn implies(self, p2: Self) -> Self {
        LogicPredicate::Implies(Box::new(self), Box::new(p2))
    }
}

impl From<&str> for LogicPredicate {
    fn from(s: &str) -> Self {
        LogicPredicate::Prop(s.to_string())
    }
}

impl std::fmt::Display for LogicPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if false {
            match self {
                LogicPredicate::True => write!(f, "true"),
                LogicPredicate::False => write!(f, "false"),

                LogicPredicate::Prop(name) => write!(f, "{}", name),
                LogicPredicate::And(p1, p2) => write!(f, "({} & {})", p1, p2),
                LogicPredicate::Or(p1, p2) => write!(f, "({} | {})", p1, p2),
                LogicPredicate::Not(p) => write!(f, "~{}", p),
                LogicPredicate::Implies(p1, p2) => write!(f, "({} -> {})", p1, p2),
            }
        } else {
            match self {
                LogicPredicate::True => write!(f, "⊤"),
                LogicPredicate::False => write!(f, "⊥"),

                LogicPredicate::Prop(name) => write!(f, "{}", name),
                LogicPredicate::And(p1, p2) => write!(f, "({} ∧ {})", p1, p2),
                LogicPredicate::Or(p1, p2) => write!(f, "({} ∨ {})", p1, p2),
                LogicPredicate::Not(p) => write!(f, "¬{}", p),
                LogicPredicate::Implies(p1, p2) => write!(f, "({} → {})", p1, p2),
            }
        }
    }
}

pub trait Propositions<P> {
    fn eval(&self, prop: &P) -> bool;
}

pub struct Transition<M: Machine>(pub M::State, pub M::Action, pub M::State);

pub struct PropositionsAllTrue;

impl<B: std::borrow::Borrow<str> + std::str::FromStr + std::fmt::Display> Propositions<B>
    for PropositionsAllTrue
{
    fn eval(&self, _: &B) -> bool {
        true
    }
}

pub trait PropMapping {
    type Prop;

    fn map(&self, name: &str) -> Option<Self::Prop>;

    fn bind<M: Machine>(&self, states: Transition<M>) -> PropositionBindings<M, Self>
    where
        Self: Sized,
        Transition<M>: Propositions<Self::Prop>,
    {
        PropositionBindings {
            props: self,
            states,
        }
    }
}

impl PropMapping for () {
    type Prop = String;

    fn map(&self, name: &str) -> Option<String> {
        Some(name.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct PropRegistry<P>(HashMap<String, P>);

impl<P: Clone> PropMapping for PropRegistry<P> {
    type Prop = P;

    fn map(&self, name: &str) -> Option<Self::Prop> {
        self.0.get(name).cloned()
    }
}

impl<P> PropRegistry<P>
where
    P: Display + Clone + PartialEq,
{
    pub fn empty() -> Self {
        Self(HashMap::new())
    }

    pub fn new<'a, T: Into<P>>(ps: impl IntoIterator<Item = T>) -> Result<Self, String> {
        let mut props = Self::empty();

        for p in ps {
            props.add(p.into())?;
        }

        Ok(props)
    }

    pub fn add(&mut self, p: P) -> Result<String, String> {
        let disp = p.to_string();
        let name = disp
            .to_lowercase()
            .replace(|ch: char| !(ch.is_alphanumeric() || ch == '_'), "_");
        let name = if name.starts_with(|ch: char| ch.is_numeric()) {
            format!("p_{name}")
        } else {
            name
        };

        if let Some(old) = self.0.insert(name.clone(), p.clone()) {
            if old != p {
                return Err(format!(
                    "Attempted to add to propmap with name collision: {disp} -> {name}"
                ));
            }
        }
        Ok(name)
    }
}

pub struct PropositionBindings<'p, M, P>
where
    M: Machine,
    P: PropMapping,
{
    props: &'p P,
    states: Transition<M>,
}

impl<'p, M, P> Propositions<String> for PropositionBindings<'p, M, P>
where
    M: Machine,
    P: PropMapping,
    Transition<M>: Propositions<P::Prop>,
{
    fn eval(&self, prop: &String) -> bool {
        let name = self
            .props
            .map(prop)
            .unwrap_or_else(|| panic!("no closure for prop: {}", prop));
        self.states.eval(&name)
    }
}

pub fn conjoin(predicates: impl IntoIterator<Item = String>) -> String {
    predicates
        .into_iter()
        .map(|p| format!("({p})"))
        .join(" && ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predicate() {
        use LogicPredicate as P;
        let p = P::or(P::not("a".into()), P::not("b".into())).not();
        assert_eq!(p.to_string(), "¬(¬a ∨ ¬b)");
        assert!(p.eval(&PropositionsAllTrue));
    }
}
