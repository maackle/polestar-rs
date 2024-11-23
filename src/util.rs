use crate::Machine;
use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

/// Return the first element of a 2-tuple
pub fn first<A, B>(tup: (A, B)) -> A {
    tup.0
}

/// Return the first element of a 2-tuple ref
pub fn first_ref<A, B>(tup: &(A, B)) -> &A {
    &tup.0
}

/// Return the second element of a 2-tuple
pub fn second<A, B>(tup: (A, B)) -> B {
    tup.1
}

/// Return the second element of a 2-tuple ref
pub fn second_ref<A, B>(tup: &(A, B)) -> &B {
    &tup.1
}

/// Swap the two items in 2-tuple
pub fn swap2<A, B>(tup: (A, B)) -> (B, A) {
    (tup.1, tup.0)
}

/// Convenience for updating state by returning an optional owned value
pub fn maybe_update<S, E>(s: &mut S, f: impl FnOnce(&S) -> (Option<S>, E)) -> E
where
    S: Sized,
{
    let (next, fx) = f(s);
    if let Some(next) = next {
        *s = next;
    }
    fx
}

/// Convenience for updating state by returning an owned value
pub fn update_replace<S, E>(s: &mut S, f: impl FnOnce(&S) -> (S, E)) -> E
where
    S: Sized,
{
    let (next, fx) = f(s);
    *s = next;
    fx
}

/// Convenience for updating state by returning an owned value
pub fn update_copy<S, E>(s: &mut S, f: impl FnOnce(S) -> (S, E)) -> E
where
    S: Sized + Copy,
{
    let (next, fx) = f(*s);
    *s = next;
    fx
}

/// When working with a HashMap whose values are the states of a Machine,
/// this function updates the state of the machine at `k` with the result of
/// applying `event` to the machine at that key.
///
/// If the state transition results in an error, the error is returned, and the
/// key is removed from the map. If the same transition is attempted again,
/// the function will return None.
pub fn transition_hashmap<K, M>(
    machine: &mut M,
    k: K,
    map: &mut HashMap<K, M::State>,
    event: M::Action,
) -> Option<Result<M::Fx, M::Error>>
where
    K: Eq + Hash,
    M: Machine,
{
    let r = machine.transition(map.remove(&k)?, event);
    match r {
        Ok((state, fx)) => {
            map.insert(k, state);
            Some(Ok(fx))
        }
        Err(e) => Some(Err(e)),
    }
}

/// When working with a BTreeMap whose values are the states of a Machine,
/// this function updates the state of the machine at `k` with the result of
/// applying `event` to the machine at that key.
///
/// If the state transition results in an error, the error is returned, and the
/// key is removed from the map. If the same transition is attempted again,
/// the function will return None.
pub fn transition_btreemap<K, M>(
    machine: &mut M,
    k: K,
    map: &mut BTreeMap<K, M::State>,
    event: M::Action,
) -> Option<Result<M::Fx, M::Error>>
where
    K: Ord,
    M: Machine,
{
    let r = machine.transition(map.remove(&k)?, event);
    match r {
        Ok((state, fx)) => {
            map.insert(k, state);
            Some(Ok(fx))
        }
        Err(e) => Some(Err(e)),
    }
}
