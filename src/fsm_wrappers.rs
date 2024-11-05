use crate::Fsm;
use proptest::prelude::{Arbitrary, BoxedStrategy, Strategy};
use proptest_derive::Arbitrary;
use std::{cell::RefCell, collections::HashMap};

#[derive(Clone, derive_more::Deref)]
pub struct FsmCell<S>(RefCell<Option<S>>);

impl<S> FsmCell<S> {
    pub fn new(s: S) -> Self {
        Self(RefCell::new(Some(s)))
    }
}

impl<S> From<S> for FsmCell<S> {
    fn from(s: S) -> Self {
        Self::new(s)
    }
}

impl<S: Fsm> FsmCell<S> {
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

impl<S: PartialEq> PartialEq for FsmCell<S> {
    fn eq(&self, other: &Self) -> bool {
        *self.0.borrow() == *other.0.borrow()
    }
}

impl<S: Eq> Eq for FsmCell<S> {}

impl<S: std::fmt::Debug> std::fmt::Debug for FsmCell<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FsmCell").field(&self.0.borrow()).finish()
    }
}

impl<S: std::hash::Hash> std::hash::Hash for FsmCell<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state)
    }
}

impl<S: Arbitrary + 'static> Arbitrary for FsmCell<S> {
    type Parameters = S::Parameters;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(p: Self::Parameters) -> Self::Strategy {
        S::arbitrary_with(p).prop_map(Self::new).boxed()
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Arbitrary,
    derive_more::Deref,
    derive_more::DerefMut,
    derive_more::From,
    derive_more::Into,
)]
pub struct FsmHashMap<K: Eq + std::hash::Hash, V: Fsm>(HashMap<K, V>);

impl<K: Eq + std::hash::Hash, V: Fsm> FsmHashMap<K, V> {
    pub fn transition_mut(&mut self, k: K, event: V::Event) -> Option<Result<V::Fx, V::Error>> {
        let r = self.0.remove(&k)?.transition(event);
        match r {
            Ok((state, fx)) => {
                self.0.insert(k, state);
                Some(Ok(fx))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

impl<K: Eq + std::hash::Hash, V: Fsm> Default for FsmHashMap<K, V> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}
