use std::{collections::HashMap, fmt::Display, marker::PhantomData, sync::Arc};

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

pub type Pair<T> = (T, T);

pub struct PropositionsAllTrue;

impl<B: std::borrow::Borrow<str> + std::str::FromStr + std::fmt::Display> Propositions<B>
    for PropositionsAllTrue
{
    fn eval(&self, _: &B) -> bool {
        true
    }
}

pub trait PropMapping<P> {
    fn map(&self, name: &str) -> Option<P>;

    fn bind<S>(&self, states: Pair<S>) -> PropositionBindings<S, Self, P>
    where
        Self: Sized,
        Pair<S>: Propositions<P>,
    {
        PropositionBindings {
            props: self,
            states,
            phantom: PhantomData,
        }
    }
}

impl PropMapping<String> for () {
    fn map(&self, name: &str) -> Option<String> {
        Some(name.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct PropRegistry<P>(HashMap<String, P>);

impl<P: Clone> PropMapping<P> for PropRegistry<P> {
    fn map(&self, name: &str) -> Option<P> {
        self.0.get(name).cloned()
    }
}

impl<P> PropRegistry<P>
where
    P: Display + Clone,
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
        let name = disp.replace(|ch: char| !(ch.is_alphanumeric() || ch == '_'), "_");
        if self.0.insert(name.clone(), p).is_some() {
            return Err(format!(
                "Attempted to add to propmap with name collision: {disp} -> {name}"
            ));
        } else {
            Ok(name)
        }
    }
}

pub struct PropositionBindings<'p, S, M, P>
where
    M: PropMapping<P>,
{
    props: &'p M,
    states: Pair<S>,
    phantom: PhantomData<P>,
}

impl<'p, S, M, P> Propositions<String> for PropositionBindings<'p, S, M, P>
where
    Pair<S>: Propositions<P>,
    M: PropMapping<P>,
    P: Display,
{
    fn eval(&self, prop: &String) -> bool {
        let name = self
            .props
            .map(prop)
            .unwrap_or_else(|| panic!("no closure for prop: {}", prop));
        self.states.eval(&name)
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
