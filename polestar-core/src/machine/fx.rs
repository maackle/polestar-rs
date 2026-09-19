//! Helpers for working with effects in transition functions.

/// Resolve a batch of effects against some state: each effect is either
/// **absorbed** — the closure changes state it captures and returns
/// `Ok(None)` — or **passed through**, possibly translated, as
/// `Ok(Some(out))`, never both. The passed-through effects are returned
/// in order, for the caller to emit as its own `Fx`.
///
/// This is the shape of a higher-level machine consuming a lower-level
/// machine's effects mid-transition (a network model turning a node's
/// `Send` into in-flight state, while `Deliver`-to-the-application
/// bubbles up), and it is deliberately the same contract as
/// [`Behavior::handle_fx`](crate::machine::Behavior::handle_fx), which
/// does the same job from outside the machine.
///
/// ```
/// use polestar_core::machine::absorb_fx;
///
/// let mut stored = vec![];
/// let out: Vec<u32> = absorb_fx([1u32, 2, 3, 4], |n| {
///     if n % 2 == 0 {
///         stored.push(n); // absorbed into state
///         Ok(None)
///     } else {
///         Ok(Some(n * 10)) // passed through, translated
///     }
/// })
/// .unwrap();
/// assert_eq!(stored, vec![2, 4]);
/// assert_eq!(out, vec![10, 30]);
/// ```
pub fn absorb_fx<A, B>(
    fx: impl IntoIterator<Item = A>,
    mut f: impl FnMut(A) -> anyhow::Result<Option<B>>,
) -> anyhow::Result<Vec<B>> {
    fx.into_iter().filter_map(|a| f(a).transpose()).collect()
}
