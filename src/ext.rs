use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
};

pub trait MapExt<K, V> {
    fn owned_update(
        &mut self,
        k: K,
        f: impl FnOnce(&mut Self, V) -> Result<V, anyhow::Error>,
    ) -> Result<(), anyhow::Error>;
}

impl<K: Debug + Hash + Eq, V> MapExt<K, V> for HashMap<K, V> {
    fn owned_update(
        &mut self,
        k: K,
        f: impl FnOnce(&mut Self, V) -> Result<V, anyhow::Error>,
    ) -> Result<(), anyhow::Error> {
        if let Some(v) = self.remove(&k) {
            let next = f(self, v)?;
            self.insert(k, next);
            Ok(())
        } else {
            Err(anyhow::anyhow!("no key {:?}", k))
        }
    }
}

impl<K: Debug + Ord, V> MapExt<K, V> for BTreeMap<K, V> {
    fn owned_update(
        &mut self,
        k: K,
        f: impl FnOnce(&mut Self, V) -> Result<V, anyhow::Error>,
    ) -> Result<(), anyhow::Error> {
        if let Some(v) = self.remove(&k) {
            let next = f(self, v)?;
            self.insert(k, next);
            Ok(())
        } else {
            Err(anyhow::anyhow!("no key {:?}", k))
        }
    }
}
