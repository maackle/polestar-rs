use std::sync::Arc;

/// A Linear Temporal Logic (LTL) formula
pub enum Ltl<S> {
    True,
    False,

    Prop(String, S),

    And(BoxLtl<S>, BoxLtl<S>),
    Or(BoxLtl<S>, BoxLtl<S>),
    Not(BoxLtl<S>),
    Implies(BoxLtl<S>, BoxLtl<S>),

    Next(BoxLtl<S>),
    Finally(BoxLtl<S>),
    Globally(BoxLtl<S>),

    Until(BoxLtl<S>, BoxLtl<S>),
    Release(BoxLtl<S>, BoxLtl<S>),
}

/// A boxed LTL formula
pub type BoxLtl<S> = Box<Ltl<S>>;

impl<S> std::fmt::Display for Ltl<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if false {
            match self {
                Ltl::True => write!(f, "true"),
                Ltl::False => write!(f, "false"),

                Ltl::Prop(name, _) => write!(f, "{}", name),
                Ltl::And(p1, p2) => write!(f, "({} & {})", p1, p2),
                Ltl::Or(p1, p2) => write!(f, "({} | {})", p1, p2),
                Ltl::Not(p) => write!(f, "~{}", p),
                Ltl::Implies(p1, p2) => write!(f, "({} -> {})", p1, p2),

                Ltl::Next(p) => write!(f, "X {}", p),
                Ltl::Finally(p) => write!(f, "F {}", p),
                Ltl::Globally(p) => write!(f, "G {}", p),

                Ltl::Until(p1, p2) => write!(f, "({} U {})", p1, p2),
                Ltl::Release(p1, p2) => write!(f, "({} R {})", p1, p2),
            }
        } else {
            match self {
                Ltl::True => write!(f, "⊤"),
                Ltl::False => write!(f, "⊥"),

                Ltl::Prop(name, _) => write!(f, "{}", name),
                Ltl::And(p1, p2) => write!(f, "({} ∧ {})", p1, p2),
                Ltl::Or(p1, p2) => write!(f, "({} ∨ {})", p1, p2),
                Ltl::Not(p) => write!(f, "¬{}", p),
                Ltl::Implies(p1, p2) => write!(f, "({} → {})", p1, p2),

                Ltl::Next(p) => write!(f, "○{}", p),
                Ltl::Finally(p) => write!(f, "◇{}", p),
                Ltl::Globally(p) => write!(f, "□{}", p),

                Ltl::Until(p1, p2) => write!(f, "({} U {})", p1, p2),
                Ltl::Release(p1, p2) => write!(f, "({} R {})", p1, p2),
            }
        }
    }
}
