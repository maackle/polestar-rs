use std::{fmt::Debug, sync::Arc};

use crate::util::first_ref;

use super::{Machine, MachineResult};

pub struct Checker<M: Machine> {
    machine: M,
    initial_predicates: Predicates<M::State>,
    make_error: Box<dyn Fn(anyhow::Error) -> M::Error>,
}

pub struct CheckerState<T> {
    predicates: Predicates<T>,
    model: T,
}

impl<M: Machine> Checker<M> {
    pub fn new(machine: M, make_error: impl Fn(anyhow::Error) -> M::Error + 'static) -> Self {
        Self {
            machine,
            initial_predicates: Predicates::new(),
            make_error: Box::new(make_error),
        }
    }

    pub fn predicate(mut self, predicate: Predicate<M::State>) -> Self {
        self.initial_predicates
            .next
            .push((format!("{:?}", predicate), predicate));
        self
    }

    pub fn check_fold(
        &self,
        initial: M::State,
        actions: impl IntoIterator<Item = M::Action>,
    ) -> Result<(), M::Error>
    where
        M: Machine,
        M::State: Clone + Debug,
    {
        let s = CheckerState {
            predicates: self.initial_predicates.clone(),
            model: initial,
        };
        let (end, _) = self.apply_actions(s, actions)?;
        end.finalize().map_err(|e| (self.make_error)(e))
    }
}

impl<T> CheckerState<T> {
    pub fn finalize(self) -> Result<(), anyhow::Error> {
        let eventuals = self
            .predicates
            .next
            .iter()
            .filter(|(_, p)| matches!(p, Predicate::Eventually(_)))
            .map(first_ref)
            .collect::<Vec<_>>();
        if !eventuals.is_empty() {
            return Err(anyhow::anyhow!(
                "Checker finalized with unsatisfied 'eventually' predicates: {eventuals:?}"
            ));
        }
        Ok(())
    }
}

// impl<M, E> Drop for Checker<M, E>
// where
//     E: std::fmt::Debug,
// {
//     fn drop(&mut self) {
//         self.finalize().unwrap();
//     }
// }

impl<M> Machine for Checker<M>
where
    M: Machine,
    M::State: Clone + Debug,
{
    type State = CheckerState<M::State>;
    type Action = M::Action;
    type Fx = M::Fx;
    type Error = M::Error;

    fn transition(&self, state: Self::State, action: Self::Action) -> MachineResult<Self> {
        let prev = state.model;
        let mut predicates = state.predicates;
        let (next, fx) = self.machine.transition(prev.clone(), action)?;
        predicates
            .step((&prev, &next))
            .map_err(|e| (self.make_error)(e))?;
        Ok((
            CheckerState {
                predicates,
                model: next,
            },
            fx,
        ))
    }
}

#[derive(Clone)]
pub struct Predicates<T> {
    next: Vec<(String, Predicate<T>)>,
}

impl<T> Predicates<T> {
    fn new() -> Self {
        Self { next: vec![] }
    }
}

impl<T: Clone + std::fmt::Debug> Predicates<T> {
    pub fn step(&mut self, state: (&T, &T)) -> Result<(), anyhow::Error> {
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
                return Err(anyhow::anyhow!(
                    "Predicate failed: '{name}'. Transition: {old:?} -> {new:?}"
                ));
            }
        }
        self.next = next;
        Ok(())
    }

    fn visit(
        next: &mut Vec<(String, Predicate<T>)>,
        negated: bool,
        name: String,
        predicate: BoxPredicate<T>,
        s: (&T, &T),
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
    struct Mach;

    impl Machine for Mach {
        type State = u8;
        type Action = u8;
        type Fx = ();
        type Error = anyhow::Error;

        fn transition(&self, _state: u8, action: u8) -> MachineResult<Self> {
            Ok((action, ()))
        }
    }

    #[test]
    fn test_checker() {
        use Predicate as P;

        tracing_subscriber::fmt::init();

        let even = P::atom("is-even".to_string(), |s: &u8| s % 2 == 0);
        let small = P::atom("single-digit".to_string(), |s: &u8| *s < 10);
        let big = P::atom("20-and-up".to_string(), |s: &u8| *s >= 20);
        let not_teens = small.clone().or(big.clone());
        let checker = Mach
            .checked(|s| s)
            .predicate(P::always(
                even.clone().implies(P::next(P::not(even.clone()))),
            ))
            .predicate(P::always(
                P::not(even.clone()).implies(P::next(even.clone())),
            ))
            .predicate(P::always(not_teens));

        checker.check_fold(0, [1, 2, 3, 23, 21]).unwrap();
    }
}
