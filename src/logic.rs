use std::{collections::HashMap, fmt::Display, sync::Arc};

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

pub type Pair<'a, T> = &'a (&'a T, &'a T);

// pub trait Propositions<P: std::fmt::Display> {
//     fn eval(&self, prop: &P) -> bool;
// }

// impl<T, P> Propositions<P> for T
// where
//     T: Propositions<P>,
//     P: std::fmt::Display,
// {
//     fn eval(pair: (&Self, &Self), prop: &P) -> bool {
//         pair.0.eval(prop)
//     }
// }

pub struct PropositionsAllTrue;

impl<B: std::borrow::Borrow<str> + std::str::FromStr + std::fmt::Display> Propositions<B>
    for PropositionsAllTrue
{
    fn eval(&self, _: &B) -> bool {
        true
    }
}

#[derive(Clone, Debug)]
pub struct PropMap<P>(Arc<HashMap<String, P>>);

impl<P> PropMap<P>
where
    P: Display + Clone,
{
    pub fn new<'a, T: Into<P>>(ps: impl IntoIterator<Item = T>) -> Self {
        let mut props = HashMap::new();

        for p in ps {
            let p: P = p.into();
            let name = p
                .to_string()
                .replace(|ch: char| !(ch.is_alphanumeric() || ch == '_'), "_");
            props.insert(name, p);
        }
        Self(Arc::new(props))
    }

    pub fn bind<'s, S>(&self, states: Pair<'s, S>) -> PropositionBindings<'s, S, P>
    where
        Pair<'s, S>: Propositions<P>,
    {
        PropositionBindings {
            props: self.clone(),
            states,
        }
    }
}

// pub struct ModelPropositions<'s, P: Display, S: Propositions<P>> {
//     props: PropMap<P>,
//     states: (&'s S, &'s S),
// }

// impl<'s, P: Display, S: Propositions<P>> Propositions<String> for ModelPropositions<'s, P, S> {
//     fn eval(&self, prop: &String) -> bool {
//         let prop = states
//             .0
//             .props
//             .0
//             .get(prop)
//             .unwrap_or_else(|| panic!("no closure for prop: {}", prop));
//         S::eval(states, prop)
//     }
// }

pub struct PropositionBindings<'s, S, P> {
    props: PropMap<P>,
    states: Pair<'s, S>,
}

impl<'s, S, P> Propositions<String> for PropositionBindings<'s, S, P>
where
    P: Display,
    Pair<'s, S>: Propositions<P>,
{
    fn eval(&self, prop: &String) -> bool {
        let prop = self
            .props
            .0
            .get(prop)
            .unwrap_or_else(|| panic!("no closure for prop: {}", prop));
        self.states.eval(prop)
    }
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
