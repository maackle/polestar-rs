use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
};

pub trait MapExt<K, V, O> {
    fn owned_update(
        &mut self,
        k: K,
        f: impl FnOnce(&mut Self, V) -> Result<(V, O), anyhow::Error>,
    ) -> Result<O, anyhow::Error>;
}

impl<K: Debug + Hash + Eq, V, O> MapExt<K, V, O> for HashMap<K, V> {
    fn owned_update(
        &mut self,
        k: K,
        f: impl FnOnce(&mut Self, V) -> Result<(V, O), anyhow::Error>,
    ) -> Result<O, anyhow::Error> {
        if let Some(v) = self.remove(&k) {
            let (next, out) = f(self, v)?;
            self.insert(k, next);
            Ok(out)
        } else {
            Err(anyhow::anyhow!("no key {:?}", k))
        }
    }
}

impl<K: Debug + Ord, V, O> MapExt<K, V, O> for BTreeMap<K, V> {
    fn owned_update(
        &mut self,
        k: K,
        f: impl FnOnce(&mut Self, V) -> Result<(V, O), anyhow::Error>,
    ) -> Result<O, anyhow::Error> {
        if let Some(v) = self.remove(&k) {
            let (next, out) = f(self, v)?;
            self.insert(k, next);
            Ok(out)
        } else {
            Err(anyhow::anyhow!("no key {:?}", k))
        }
    }
}
