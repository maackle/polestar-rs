use std::sync::Arc;

use super::{Machine, MachineResult};

pub struct Checker<M: Machine> {
    state: M,
    predicates: Predicates<M, M::Error>,
}

impl<M: Machine> Checker<M> {
    pub fn new(state: M, make_error: impl Fn(anyhow::Error) -> M::Error + 'static) -> Self {
        Self {
            state,
            predicates: Predicates::new(make_error),
        }
    }

    pub fn with(mut self, predicate: Predicate<M>) -> Self {
        self.predicates
            .next
            .push((format!("{:?}", predicate), predicate));
        self
    }
}

impl<M> Machine for Checker<M>
where
    M: Machine + Clone + std::fmt::Debug,
{
    type Action = M::Action;
    type Fx = M::Fx;
    type Error = M::Error;

    fn transition(mut self, action: Self::Action) -> MachineResult<Self> {
        let prev = self.state.clone();
        let (next, fx) = self.state.transition(action)?;
        self.predicates = self.predicates.step((&prev, &next))?;
        self.state = next;
        Ok((self, fx))
    }
}

pub struct Predicates<M, E> {
    next: Vec<(String, Predicate<M>)>,
    make_error: Box<dyn Fn(anyhow::Error) -> E>,
}

impl<M, E> Predicates<M, E> {
    fn new(make_error: impl Fn(anyhow::Error) -> E + 'static) -> Self {
        Self {
            next: vec![],
            make_error: Box::new(make_error),
        }
    }
}

impl<M: Clone + std::fmt::Debug, E> Predicates<M, E> {
    pub fn step(mut self, state: (&M, &M)) -> Result<Self, E> {
        let mut next = vec![];
        tracing::debug!("");
        tracing::debug!("------------------------------------------------");
        tracing::debug!("STEP: {:?} -> {:?}", state.0, state.1);

        let now = self.next.drain(..).collect::<Vec<_>>();
        for (name, predicate) in now {
            tracing::debug!("");
            tracing::debug!("visiting {predicate:?}");
            if !Self::visit(
                &mut next,
                false,
                name.clone(),
                Box::new(predicate.clone()),
                state,
            ) {
                let (old, new) = state;
                return Err((self.make_error)(anyhow::anyhow!(
                    "Predicate failed: '{name}'   Transition: {old:?} -> {new:?}"
                )));
            }
        }
        self.next = next;
        Ok(self)
    }

    fn visit(
        next: &mut Vec<(String, Predicate<M>)>,
        negated: bool,
        name: String,
        predicate: BoxPredicate<M>,
        s: (&M, &M),
    ) -> bool {
        use Predicate::*;
        let out = match *predicate.clone() {
            Next(p) => {
                next.push((name, *p));
                true
            }

            // Eventually(Eventually(p)) => Self::visit(next, negated, Eventually(p), s),
            Eventually(p) => {
                if !Self::visit(next, negated, name.clone(), p.clone(), s) {
                    next.push((name, Eventually(p.clone()).negate(negated)));
                }
                true
            }

            // Always(Always(p)) => Self::visit(negated, Always(p), s),
            Always(p) => {
                next.push((name.clone(), Always(p.clone()).negate(negated)));
                Self::visit(next, negated, name.clone(), p.clone(), s)
            }

            Not(p) => Self::visit(next, !negated, name, p, s),

            And(p1, p2) => {
                Self::visit(next, negated, name.clone(), p1, s)
                    && Self::visit(next, negated, name, p2, s)
            }

            Or(p1, p2) => {
                Self::visit(next, negated, name.clone(), p1, s)
                    || Self::visit(next, negated, name, p2, s)
            }

            Implies(p1, p2) => {
                !Self::visit(next, negated, name.clone(), p1, s)
                    || Self::visit(next, negated, name, p2, s)
            }
            Atom(_, f) => {
                if negated {
                    !f(s.0, s.1)
                } else {
                    f(s.0, s.1)
                }
            }
        };
        if negated {
            tracing::debug!("NEG {predicate:?} = {out}");
        } else {
            tracing::debug!("    {predicate:?} = {out}");
        }
        out
    }
}

pub type BoxPredicate<M> = Box<Predicate<M>>;

#[derive(Clone)]
pub enum Predicate<M> {
    Atom(String, Arc<dyn Fn(&M, &M) -> bool>),
    And(BoxPredicate<M>, BoxPredicate<M>),
    Or(BoxPredicate<M>, BoxPredicate<M>),
    Not(BoxPredicate<M>),
    Implies(BoxPredicate<M>, BoxPredicate<M>),

    Next(BoxPredicate<M>),
    Eventually(BoxPredicate<M>),
    Always(BoxPredicate<M>),
}

impl<M> Predicate<M> {
    pub fn next(self) -> Self {
        Self::Next(Box::new(self))
    }

    pub fn eventually(self) -> Self {
        Self::Eventually(Box::new(self))
    }

    pub fn always(self) -> Self {
        Self::Always(Box::new(self))
    }

    pub fn not(self) -> Self {
        Self::Not(Box::new(self))
    }

    pub fn negate(self, negated: bool) -> Self {
        if negated {
            self.not()
        } else {
            self
        }
    }

    pub fn implies(self: Predicate<M>, p2: Predicate<M>) -> Self {
        Self::Implies(Box::new(self), Box::new(p2))
    }

    pub fn and(self: Predicate<M>, p2: Predicate<M>) -> Self {
        Self::And(Box::new(self), Box::new(p2))
    }

    pub fn or(self: Predicate<M>, p2: Predicate<M>) -> Self {
        Self::Or(Box::new(self), Box::new(p2))
    }

    pub fn atom(name: String, f: impl Fn(&M) -> bool + 'static) -> Self {
        Self::atom2(name, move |_, b| f(b))
    }

    pub fn atom2(name: String, f: impl Fn(&M, &M) -> bool + 'static) -> Self {
        assert!(!name.contains(' '), "no spaces allowed in predicate names");
        Self::Atom(name, Arc::new(f))
    }
}

impl<M> std::fmt::Debug for Predicate<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if true {
            match self {
                Predicate::Atom(name, _) => write!(f, "{}", name),
                Predicate::And(p1, p2) => write!(f, "({:?} & {:?})", p1, p2),
                Predicate::Or(p1, p2) => write!(f, "({:?} | {:?})", p1, p2),
                Predicate::Not(p) => write!(f, "~{:?}", p),
                Predicate::Implies(p1, p2) => write!(f, "({:?} -> {:?})", p1, p2),

                Predicate::Next(p) => write!(f, "X {:?}", p),
                Predicate::Eventually(p) => write!(f, "F {:?}", p),
                Predicate::Always(p) => write!(f, "G {:?}", p),
            }
        } else {
            match self {
                Predicate::Atom(name, _) => write!(f, "{}", name),
                Predicate::And(p1, p2) => write!(f, "({:?} ∧ {:?})", p1, p2),
                Predicate::Or(p1, p2) => write!(f, "({:?} ∨ {:?})", p1, p2),
                Predicate::Not(p) => write!(f, "¬{:?}", p),
                Predicate::Implies(p1, p2) => write!(f, "({:?} → {:?})", p1, p2),

                Predicate::Next(p) => write!(f, "○{:?}", p),
                Predicate::Eventually(p) => write!(f, "◇{:?}", p),
                Predicate::Always(p) => write!(f, "□{:?}", p),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Mach {
        state: u8,
    }

    impl Machine for Mach {
        type Action = u8;
        type Fx = ();
        type Error = String;

        fn transition(mut self, action: u8) -> MachineResult<Self> {
            self.state = action;
            Ok((self, ()))
        }
    }

    #[test]
    fn test_checker() {
        use Predicate as P;

        tracing_subscriber::fmt::init();

        let even = P::atom("is-even".to_string(), |s: &Mach| s.state % 2 == 0);
        let small = P::atom("single-digit".to_string(), |s: &Mach| s.state < 10);
        let big = P::atom("20-and-up".to_string(), |s: &Mach| s.state >= 20);
        let not_teens = small.clone().or(big.clone());
        let checker = Checker::new(Mach { state: 0 }, |s| s.to_string())
            .with(P::always(
                even.clone().implies(P::next(P::not(even.clone()))),
            ))
            .with(P::always(
                P::not(even.clone()).implies(P::next(even.clone())),
            ))
            .with(P::always(not_teens));

        checker
            .transition_(1)
            .unwrap()
            .transition_(2)
            .unwrap()
            .transition_(3)
            .unwrap()
            .transition_(22)
            .unwrap()
            .transition_(21)
            .unwrap();
    }
}
