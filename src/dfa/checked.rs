use std::{fmt::Debug, sync::Arc};

use im::{vector, Vector};

use crate::util::first_ref;

use super::{Machine, MachineResult};

pub struct Checker<M: Machine> {
    machine: M,
    initial_predicates: Predicates<M::State>,
}

#[derive(Clone)]
pub struct CheckerState<S, A> {
    predicates: Predicates<S>,
    pub state: S,
    path: im::Vector<A>,
}

// XXX: whoa there! be careful with this.
impl<M, A> PartialEq for CheckerState<M, A>
where
    M: PartialEq,
    A: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

impl<M, A> Eq for CheckerState<M, A>
where
    M: Eq,
    A: Eq,
{
}

impl<M, A> std::hash::Hash for CheckerState<M, A>
where
    M: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.state.hash(state)
    }
}

#[derive(Debug, derive_more::From)]
#[cfg_attr(test, derive(derive_more::Unwrap))]
pub enum CheckerError<A: Clone, E> {
    Predicate(PredicateError<A>),
    #[from]
    Machine(E),
}

#[derive(Debug)]
pub struct PredicateError<A: Clone> {
    pub error: String,
    pub path: im::Vector<A>,
}

impl<M: Machine> Checker<M> {
    pub fn new(machine: M) -> Self {
        Self {
            machine,
            initial_predicates: Predicates::new(),
        }
    }

    pub fn predicate(mut self, predicate: Predicate<M::State>) -> Self {
        self.initial_predicates
            .next
            .push_back((format!("{:?}", predicate), predicate));
        self
    }

    pub fn initial(&self, s: M::State) -> CheckerState<M::State, M::Action>
    where
        M::State: Clone + Debug,
        M::Action: Clone + Debug,
    {
        CheckerState {
            predicates: self.initial_predicates.clone(),
            state: s,
            path: vector![],
        }
    }

    pub fn check_fold(
        &self,
        initial: M::State,
        actions: impl IntoIterator<Item = M::Action>,
    ) -> Result<(), CheckerError<M::Action, M::Error>>
    where
        M::State: Clone + Debug,
        M::Action: Clone + Debug,
    {
        let s = self.initial(initial);
        let (end, _) = self.apply_actions(s, actions)?;
        end.finalize()
            .map_err(|(error, path)| CheckerError::Predicate(PredicateError { error, path }))
    }
}

impl<M, A> CheckerState<M, A> {
    pub fn finalize(self) -> Result<(), (String, im::Vector<A>)> {
        let eventuals = self
            .predicates
            .next
            .iter()
            .filter(|(_, p)| matches!(p, Predicate::Eventually(_)))
            .map(first_ref)
            .collect::<Vec<_>>();
        if !eventuals.is_empty() {
            return Err((
                format!(
                    "Checker finalized with unsatisfied 'eventually' predicates: {eventuals:?}"
                ),
                self.path,
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
    M::Action: Clone + Debug,
{
    type State = CheckerState<M::State, M::Action>;
    type Action = M::Action;
    type Fx = M::Fx;
    type Error = CheckerError<M::Action, M::Error>;

    fn transition(&self, state: Self::State, action: Self::Action) -> MachineResult<Self> {
        let prev = state.state;
        let mut predicates = state.predicates;
        let mut path = state.path;
        let (next, fx) = self.machine.transition(prev.clone(), action.clone())?;
        path.push_back(action);
        predicates.step((&prev, &next)).map_err(|error| {
            CheckerError::Predicate(PredicateError {
                error,
                path: path.clone(),
            })
        })?;
        Ok((
            CheckerState {
                predicates,
                state: next,
                path,
            },
            fx,
        ))
    }

    fn is_terminal(&self, s: &Self::State) -> bool {
        self.machine.is_terminal(&s.state)
    }
}

#[derive(Clone)]
pub struct Predicates<T> {
    next: im::Vector<(String, Predicate<T>)>,
}

impl<T> Predicates<T> {
    fn new() -> Self {
        Self {
            next: im::vector![],
        }
    }
}

impl<T: Clone + std::fmt::Debug> Predicates<T> {
    pub fn step(&mut self, state: (&T, &T)) -> Result<(), String> {
        let mut next = vector![];
        let mut now = vector![];
        tracing::debug!("");
        tracing::debug!("------------------------------------------------");
        tracing::debug!("STEP: {:?} -> {:?}", state.0, state.1);

        std::mem::swap(&mut self.next, &mut now);
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
                return Err(format!(
                    "Predicate failed: '{name}'. Transition: {old:?} -> {new:?}"
                ));
            }
        }
        self.next = next;
        Ok(())
    }

    // TODO: include the Machine?
    fn visit(
        next: &mut Vector<(String, Predicate<T>)>,
        negated: bool,
        name: String,
        predicate: BoxPredicate<T>,
        s: (&T, &T),
    ) -> bool {
        use Predicate::*;
        let out = match *predicate.clone() {
            Next(p) => {
                next.push_back((name, *p));
                true
            }

            // Eventually(Eventually(p)) => Self::visit(next, negated, Eventually(p), s),
            Eventually(p) => {
                if !Self::visit(next, negated, name.clone(), p.clone(), s) {
                    next.push_back((name, Eventually(p.clone()).negate(negated)));
                }
                true
            }

            // Always(Always(p)) => Self::visit(negated, Always(p), s),
            Always(p) => {
                next.push_back((name.clone(), Always(p.clone()).negate(negated)));
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

impl<M> Clone for Predicate<M> {
    fn clone(&self) -> Self {
        match self {
            Self::Atom(name, f) => Self::Atom(name.clone(), f.clone()),
            Self::And(p1, p2) => Self::And(p1.clone(), p2.clone()),
            Self::Or(p1, p2) => Self::Or(p1.clone(), p2.clone()),
            Self::Not(p) => Self::Not(p.clone()),
            Self::Implies(p1, p2) => Self::Implies(p1.clone(), p2.clone()),
            Self::Next(p) => Self::Next(p.clone()),
            Self::Eventually(p) => Self::Eventually(p.clone()),
            Self::Always(p) => Self::Always(p.clone()),
        }
    }
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
        // assert!(!name.contains(' '), "no spaces allowed in predicate names");
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

        fn is_terminal(&self, _: &Self::State) -> bool {
            false
        }
    }

    #[test]
    fn test_checker() {
        use Predicate as P;

        tracing_subscriber::fmt::init();

        let even = P::atom("is-even".to_string(), |s: &u8| s % 2 == 0);
        let small = P::atom("single-digit".to_string(), |s: &u8| *s < 10);
        let big = P::atom("20-and-up".to_string(), |s: &u8| *s >= 20);
        let reallybig = P::atom("100-and-up".to_string(), |s: &u8| *s >= 100);
        let not_teens = small.clone().or(big.clone());
        let checker = Mach
            .checked()
            .predicate(P::always(
                even.clone().implies(P::next(P::not(even.clone()))),
            ))
            .predicate(P::always(
                P::not(even.clone()).implies(P::next(even.clone())),
            ))
            .predicate(P::always(not_teens))
            .predicate(P::eventually(reallybig));

        checker.check_fold(0, [1, 2, 3, 108, 21]).unwrap();

        let err = checker.check_fold(0, [1, 2, 3, 23, 21]).unwrap_err();
        assert_eq!(err.unwrap_predicate().path, vector![1, 2, 3, 23]);

        let err = checker.check_fold(1, [2, 12, 33]).unwrap_err();
        assert_eq!(err.unwrap_predicate().path, vector![2, 12]);
    }
}
