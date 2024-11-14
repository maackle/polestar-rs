use crate::Machine;
use proptest_derive::Arbitrary;
use std::collections::HashMap;

#[deprecated = "seems silly"]
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
pub struct FsmHashMap<K: Eq + std::hash::Hash, V>(HashMap<K, V>);

impl<K: Eq + std::hash::Hash, V: Machine> FsmHashMap<K, V> {
    pub fn transition_mut(&mut self, k: K, event: V::Action) -> Option<Result<V::Fx, V::Error>> {
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

impl<K: Eq + std::hash::Hash, V: Machine> Default for FsmHashMap<K, V> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}
