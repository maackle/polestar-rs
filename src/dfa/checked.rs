use std::{fmt::Debug, sync::Arc};

use im::{vector, Vector};

use crate::util::first_ref;

use super::{Machine, MachineResult};

#[derive(Debug)]
pub struct Checker<M: Machine> {
    machine: M,
    initial_predicates: Vec<Predicate<M, M::State>>,
}

pub struct CheckerState<M: Machine> {
    predicates: Predicates<M, M::State>,
    pub state: M::State,
    path: im::Vector<M::Action>,
}

impl<M> Clone for CheckerState<M>
where
    M: Machine,
    M::State: Clone,
    M::Action: Clone,
{
    fn clone(&self) -> Self {
        Self {
            predicates: self.predicates.clone(),
            state: self.state.clone(),
            path: self.path.clone(),
        }
    }
}

// XXX: whoa there! be careful with this.
impl<M> PartialEq for CheckerState<M>
where
    M: Machine,
    M::State: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

impl<M> Eq for CheckerState<M>
where
    M: Machine,
    M::State: Eq,
{
}

impl<M> std::hash::Hash for CheckerState<M>
where
    M: Machine,
    M::State: std::hash::Hash,
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
            initial_predicates: Vec::new(),
        }
    }

    pub fn with_predicates(
        mut self,
        predicates: impl IntoIterator<Item = Predicate<M, M::State>>,
    ) -> Self {
        self.initial_predicates.extend(predicates.into_iter());
        self
    }

    pub fn initial(&self, s: M::State) -> CheckerState<M>
    where
        M::State: Clone + Debug,
        M::Action: Clone + Debug,
    {
        CheckerState {
            predicates: Predicates::new(self.initial_predicates.clone().into_iter()),
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
        M: Debug,
        M::State: Clone + Debug,
        M::Action: Clone + Debug,
    {
        let s = self.initial(initial);
        let (end, _) = self.apply_actions(s, actions)?;
        end.finalize()
            .map_err(|(error, path)| CheckerError::Predicate(PredicateError { error, path }))
    }

    pub fn get_predicates(&self) -> &[Predicate<M, M::State>] {
        &self.initial_predicates
    }
}

impl<M: Machine> CheckerState<M> {
    pub fn finalize(self) -> Result<(), (String, im::Vector<M::Action>)> {
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
    M: Machine + Debug,
    M::State: Clone + Debug,
    M::Action: Clone + Debug,
{
    type State = CheckerState<M>;
    type Action = M::Action;
    type Fx = M::Fx;
    type Error = CheckerError<M::Action, M::Error>;

    fn transition(&self, state: Self::State, action: Self::Action) -> MachineResult<Self> {
        let prev = state.state;
        let mut predicates = state.predicates;
        let mut path = state.path;
        let (next, fx) = self.machine.transition(prev.clone(), action.clone())?;
        path.push_back(action);
        predicates
            .step((&self.machine, &prev, &next))
            .map_err(|error| {
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

#[derive(Default, derive_more::From)]
pub struct Predicates<C, S> {
    next: im::Vector<(String, Predicate<C, S>)>,
}

impl<C, S> Clone for Predicates<C, S> {
    fn clone(&self) -> Self {
        Self {
            next: self.next.clone(),
        }
    }
}

impl<C, S> Predicates<C, S> {
    fn new(ps: impl Iterator<Item = Predicate<C, S>>) -> Self {
        Self {
            next: ps.map(|p| (format!("{:?}", p), p)).collect(),
        }
    }
}

impl<C: std::fmt::Debug, S: std::fmt::Debug> Predicates<C, S> {
    pub fn step(&mut self, state: (&C, &S, &S)) -> Result<(), String> {
        let mut next = vector![];
        let mut now = vector![];
        tracing::debug!("");
        tracing::debug!("------------------------------------------------");
        tracing::debug!("STEP: {:?} -> {:?}", state.1, state.2);

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
                let (ctx, old, new) = state;
                return Err(format!(
                    "Predicate failed: '{name}'. Context: {ctx:?}. Transition: {old:?} -> {new:?}"
                ));
            }
        }
        self.next = next;
        Ok(())
    }

    // TODO: include the Machine?
    fn visit(
        next: &mut Vector<(String, Predicate<C, S>)>,
        negated: bool,
        name: String,
        predicate: BoxPredicate<C, S>,
        s: (&C, &S, &S),
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
                    !f(s.0, s.1, s.2)
                } else {
                    f(s.0, s.1, s.2)
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

pub type BoxPredicate<C, S> = Box<Predicate<C, S>>;

pub enum Predicate<C, S> {
    Atom(String, Arc<dyn Fn(&C, &S, &S) -> bool>),
    And(BoxPredicate<C, S>, BoxPredicate<C, S>),
    Or(BoxPredicate<C, S>, BoxPredicate<C, S>),
    Not(BoxPredicate<C, S>),
    Implies(BoxPredicate<C, S>, BoxPredicate<C, S>),

    Next(BoxPredicate<C, S>),
    Eventually(BoxPredicate<C, S>),
    Always(BoxPredicate<C, S>),
}

impl<C, S> Clone for Predicate<C, S> {
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

impl<C, S> Predicate<C, S> {
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

    pub fn implies(self: Predicate<C, S>, p2: Predicate<C, S>) -> Self {
        Self::Implies(Box::new(self), Box::new(p2))
    }

    pub fn and(self: Predicate<C, S>, p2: Predicate<C, S>) -> Self {
        Self::And(Box::new(self), Box::new(p2))
    }

    pub fn or(self: Predicate<C, S>, p2: Predicate<C, S>) -> Self {
        Self::Or(Box::new(self), Box::new(p2))
    }

    pub fn atom(name: String, f: impl Fn(&S) -> bool + 'static) -> Self {
        Self::atom3(name, move |_, _, s| f(s))
    }

    pub fn atom2(name: String, f: impl Fn(&S, &S) -> bool + 'static) -> Self {
        Self::atom3(name, move |_, a, b| f(a, b))
    }

    pub fn atom3(name: String, f: impl Fn(&C, &S, &S) -> bool + 'static) -> Self {
        Self::Atom(name, Arc::new(f))
    }
}

impl<C, S> std::fmt::Debug for Predicate<C, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if false {
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
        let checker = Mach.checked().with_predicates([
            P::always(even.clone().implies(P::next(P::not(even.clone())))),
            P::always(P::not(even.clone()).implies(P::next(even.clone()))),
            P::always(not_teens),
            P::eventually(reallybig),
        ]);

        checker.check_fold(0, [1, 2, 3, 108, 21]).unwrap();

        let err = checker.check_fold(0, [1, 2, 3, 23, 21]).unwrap_err();
        assert_eq!(err.unwrap_predicate().path, vector![1, 2, 3, 23]);

        let err = checker.check_fold(1, [2, 12, 33]).unwrap_err();
        assert_eq!(err.unwrap_predicate().path, vector![2, 12]);
    }
}
