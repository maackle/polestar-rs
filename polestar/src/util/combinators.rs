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

pub fn product2<A, B, IB>(a: impl IntoIterator<Item = A>, b: IB) -> impl Iterator<Item = (A, B)>
where
    A: Clone,
    IB: IntoIterator<Item = B>,
    IB::IntoIter: Clone,
{
    use itertools::Itertools;
    a.into_iter().cartesian_product(b.into_iter())
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
