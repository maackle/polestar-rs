use std::{collections::HashMap, fmt::Display};

use itertools::Itertools;

use crate::Machine;

#[cfg(feature = "ltl3ba")]
mod ltl3ba_parser;
#[cfg(feature = "ltl3ba")]
pub use ltl3ba_parser::*;

#[cfg(feature = "ltl3ba")]
mod propositions;
#[cfg(feature = "ltl3ba")]
pub use propositions::*;

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

pub struct Transition<M: Machine>(pub M::State, pub M::Action, pub M::State);

#[cfg(test)]
mod tests {
    use super::*;

    struct PropositionsAllTrue;

    impl<B: std::borrow::Borrow<str> + std::str::FromStr + std::fmt::Display> Propositions<B>
        for PropositionsAllTrue
    {
        fn eval(&self, _: &B) -> bool {
            true
        }
    }

    #[test]
    fn test_predicate() {
        use LogicPredicate as P;
        let p = P::or(P::not("a".into()), P::not("b".into())).not();
        assert_eq!(p.to_string(), "¬(¬a ∨ ¬b)");
        assert!(p.eval(&PropositionsAllTrue));
    }
}
