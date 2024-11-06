use crate::Fsm;
use proptest_derive::Arbitrary;
use std::collections::BTreeMap;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Arbitrary,
    derive_more::Constructor,
    derive_more::Deref,
    derive_more::DerefMut,
    derive_more::From,
    derive_more::Into,
    derive_more::IntoIterator,
)]
pub struct FsmBTreeMap<K: Ord, V>(BTreeMap<K, V>);

impl<K: Ord, V: Fsm> FsmBTreeMap<K, V> {
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

impl<K: Ord, V: Fsm> Default for FsmBTreeMap<K, V> {
    fn default() -> Self {
        Self(BTreeMap::default())
    }
}
