use crate::Fsm;
use proptest::prelude::{Arbitrary, BoxedStrategy, Strategy};
use std::cell::RefCell;

pub struct FsmCell<S: Fsm>(RefCell<Option<S>>);

impl<S: Fsm> FsmCell<S>
where
    S::Error: Clone,
{
    pub fn new(state: S) -> Self {
        Self(RefCell::new(Some(state)))
    }

    pub fn transition_mut(&mut self, event: S::Event) -> Option<Result<S::Fx, S::Error>> {
        match self.0.take()?.transition(event) {
            Err(e) => Some(Err(e)),
            Ok((state, fx)) => {
                self.0.replace(Some(state));
                Some(Ok(fx))
            }
        }
    }
}

impl<S> PartialEq for FsmCell<S> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<S> Eq for FsmCell<S> {}

impl<S: Debug> Debug for FsmCell<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.read(|s| f.debug_tuple("FsmCell").field(s).finish())
    }
}

impl<S: std::hash::Hash> std::hash::Hash for FsmCell<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.read(|s| s.hash(state))
    }
}

impl<S: Arbitrary + 'static> Arbitrary for FsmCell<S> {
    type Parameters = S::Parameters;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(p: Self::Parameters) -> Self::Strategy {
        S::arbitrary_with(p).prop_map(Self::new).boxed()
    }
}
