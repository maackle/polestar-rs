//! Types related to logic propositions and statements,
//! used in LTL model checking.

use std::{collections::HashMap, fmt::Display, hash::Hash};

use itertools::Itertools;

use crate::Machine;

#[cfg(feature = "ltl3ba")]
mod ltl3ba_parser;

#[cfg(feature = "ltl3ba")]
mod propositions;
#[cfg(feature = "ltl3ba")]
pub use propositions::*;

/// A propositional logic statement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogicStatement {
    /// True
    True,
    /// False
    False,

    /// An atomic proposition
    Prop(String),

    /// Logical AND
    And(BoxPredicate, BoxPredicate),
    /// Logical OR
    Or(BoxPredicate, BoxPredicate),
    /// Logical NOT
    Not(BoxPredicate),
    /// Logical implication
    Implies(BoxPredicate, BoxPredicate),
}

type BoxPredicate = Box<LogicStatement>;

#[allow(clippy::should_implement_trait)]
impl LogicStatement {
    /// Evaluate the logic statement against the set of propositions
    pub fn eval(&self, props: &impl EvaluatePropositions<String>) -> bool {
        match self {
            LogicStatement::True => true,
            LogicStatement::False => false,

            LogicStatement::Prop(name) => props.evaluate(name),
            LogicStatement::And(p1, p2) => p1.eval(props) && p2.eval(props),
            LogicStatement::Or(p1, p2) => p1.eval(props) || p2.eval(props),
            LogicStatement::Not(p) => !p.eval(props),
            LogicStatement::Implies(p1, p2) => !p1.eval(props) || p2.eval(props),
        }
    }

    /// Combine with NOT
    pub fn not(self) -> Self {
        LogicStatement::Not(Box::new(self))
    }

    /// Combine with AND
    pub fn and(self, p2: Self) -> Self {
        LogicStatement::And(Box::new(self), Box::new(p2))
    }

    /// Combine with OR
    pub fn or(self, p2: Self) -> Self {
        LogicStatement::Or(Box::new(self), Box::new(p2))
    }

    /// Combine with implication
    pub fn implies(self, p2: Self) -> Self {
        LogicStatement::Implies(Box::new(self), Box::new(p2))
    }
}

impl From<&str> for LogicStatement {
    fn from(s: &str) -> Self {
        LogicStatement::Prop(s.to_string())
    }
}

impl std::fmt::Display for LogicStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if false {
            match self {
                LogicStatement::True => write!(f, "true"),
                LogicStatement::False => write!(f, "false"),

                LogicStatement::Prop(name) => write!(f, "{}", name),
                LogicStatement::And(p1, p2) => write!(f, "({} & {})", p1, p2),
                LogicStatement::Or(p1, p2) => write!(f, "({} | {})", p1, p2),
                LogicStatement::Not(p) => write!(f, "~{}", p),
                LogicStatement::Implies(p1, p2) => write!(f, "({} -> {})", p1, p2),
            }
        } else {
            match self {
                LogicStatement::True => write!(f, "⊤"),
                LogicStatement::False => write!(f, "⊥"),

                LogicStatement::Prop(name) => write!(f, "{}", name),
                LogicStatement::And(p1, p2) => write!(f, "({} ∧ {})", p1, p2),
                LogicStatement::Or(p1, p2) => write!(f, "({} ∨ {})", p1, p2),
                LogicStatement::Not(p) => write!(f, "¬{}", p),
                LogicStatement::Implies(p1, p2) => write!(f, "({} → {})", p1, p2),
            }
        }
    }
}

/// A transition between state, including the previous and next state,
/// and the action which caused the transition.
///
/// [`Propositions<P>`] must be implemented over `Transition<T>` in order
/// to use LTL formulae in [`crate::model_checker::ModelChecker`].
#[derive(
    derive_bounded::Clone, derive_bounded::Debug, derive_bounded::PartialEq, derive_bounded::Eq,
)]
#[bounded_to(M::State, M::Action)]
pub struct Transition<M: Machine>(pub M::State, pub M::Action, pub M::State);

impl<M: Machine> Hash for Transition<M> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
        self.2.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PropositionsAllTrue;

    impl<B: std::borrow::Borrow<str> + std::str::FromStr + std::fmt::Display>
        EvaluatePropositions<B> for PropositionsAllTrue
    {
        fn evaluate(&self, _: &B) -> bool {
            true
        }
    }

    #[test]
    fn test_predicate() {
        use LogicStatement as P;
        let p = P::or(P::not("a".into()), P::not("b".into())).not();
        assert_eq!(p.to_string(), "¬(¬a ∨ ¬b)");
        assert!(p.eval(&PropositionsAllTrue));
    }
}
