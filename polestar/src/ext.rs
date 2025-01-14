//! An extension trait for working with maps in transition functions

use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
};

/// An extension trait for various Map types, which is useful in
/// [`Machine::transition`][crate::machine::Machine::transition] functions,
/// where it is common for the machine's State to contain a map where one key
/// at a time needs to be updated.
pub trait MapExt<K, V, O> {
    /// Update the value at `k` with a function taking the owned value and returning a new value.
    fn owned_update(
        &mut self,
        k: K,
        f: impl FnOnce(&mut Self, V) -> Result<(V, O), anyhow::Error>,
    ) -> Result<O, anyhow::Error>;

    /// Update the value at `k` with a function taking the owned value and returning a new value,
    /// or insert a new value if `k` is not present.
    fn owned_upsert(
        &mut self,
        k: K,
        u: impl FnOnce(&Self) -> Result<V, anyhow::Error>,
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
            Err(anyhow::anyhow!("owned_update: no key {:?}", k))
        }
    }

    fn owned_upsert(
        &mut self,
        k: K,
        u: impl FnOnce(&Self) -> Result<V, anyhow::Error>,
        f: impl FnOnce(&mut Self, V) -> Result<(V, O), anyhow::Error>,
    ) -> Result<O, anyhow::Error> {
        if let Some(v) = self.remove(&k) {
            let (next, out) = f(self, v)?;
            self.insert(k, next);
            Ok(out)
        } else {
            let (v, o) = f(self, u(self)?)?;
            self.insert(k, v);
            Ok(o)
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
            Err(anyhow::anyhow!("owned_update: no key {:?}", k))
        }
    }

    fn owned_upsert(
        &mut self,
        k: K,
        u: impl FnOnce(&Self) -> Result<V, anyhow::Error>,
        f: impl FnOnce(&mut Self, V) -> Result<(V, O), anyhow::Error>,
    ) -> Result<O, anyhow::Error> {
        if let Some(v) = self.remove(&k) {
            let (next, out) = f(self, v)?;
            self.insert(k, next);
            Ok(out)
        } else {
            let (v, o) = f(self, u(self)?)?;
            self.insert(k, v);
            Ok(o)
        }
    }
}

impl<K: Debug + Hash + Eq + Clone, V: Clone, O> MapExt<K, V, O> for im::HashMap<K, V> {
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
            Err(anyhow::anyhow!("owned_update: no key {:?}", k))
        }
    }

    fn owned_upsert(
        &mut self,
        k: K,
        u: impl FnOnce(&Self) -> Result<V, anyhow::Error>,
        f: impl FnOnce(&mut Self, V) -> Result<(V, O), anyhow::Error>,
    ) -> Result<O, anyhow::Error> {
        if let Some(v) = self.remove(&k) {
            let (next, out) = f(self, v)?;
            self.insert(k, next);
            Ok(out)
        } else {
            let (v, o) = f(self, u(self)?)?;
            self.insert(k, v);
            Ok(o)
        }
    }
}

impl<K: Debug + Ord + Clone, V: Clone, O> MapExt<K, V, O> for im::OrdMap<K, V> {
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
            Err(anyhow::anyhow!("owned_update: no key {:?}", k))
        }
    }

    fn owned_upsert(
        &mut self,
        k: K,
        u: impl FnOnce(&Self) -> Result<V, anyhow::Error>,
        f: impl FnOnce(&mut Self, V) -> Result<(V, O), anyhow::Error>,
    ) -> Result<O, anyhow::Error> {
        if let Some(v) = self.remove(&k) {
            let (next, out) = f(self, v)?;
            self.insert(k, next);
            Ok(out)
        } else {
            let (v, o) = f(self, u(self)?)?;
            self.insert(k, v);
            Ok(o)
        }
    }
}
