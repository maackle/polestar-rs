use crate::Machine;
use std::collections::{BTreeMap, HashMap};

pub trait HashMapExt<K: std::hash::Hash + Eq, V: Machine> {
    fn transition_mut(&mut self, k: K, event: V::Action) -> Option<Result<V::Fx, V::Error>>;
}

impl<K: std::hash::Hash + Eq, V: Machine> HashMapExt<K, V> for HashMap<K, V> {
    fn transition_mut(&mut self, k: K, event: V::Action) -> Option<Result<V::Fx, V::Error>> {
        let r = self.remove(&k)?.transition(event);
        match r {
            Ok((state, fx)) => {
                self.insert(k, state);
                Some(Ok(fx))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

pub trait BTreeMapExt<K: Ord, V: Machine> {
    fn transition_mut(&mut self, k: K, event: V::Action) -> Option<Result<V::Fx, V::Error>>;
}

impl<K: Ord, V: Machine> BTreeMapExt<K, V> for BTreeMap<K, V> {
    fn transition_mut(&mut self, k: K, event: V::Action) -> Option<Result<V::Fx, V::Error>> {
        let r = self.remove(&k)?.transition(event);
        match r {
            Ok((state, fx)) => {
                self.insert(k, state);
                Some(Ok(fx))
            }
            Err(e) => Some(Err(e)),
        }
    }
}
