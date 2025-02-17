//! Representations of time suitable for deterministic model checking.
//!
//! This includes both discrete and continuous time.

use human_repr::HumanDuration;
use num_traits::*;
use std::{
    fmt::Display,
    marker::PhantomData,
    ops::{Mul, Sub},
    time::{Duration, Instant},
};

use crate::id::UpTo;

/// Types which can represent an interval of time as needed by a model.
pub trait TimeInterval:
    Clone
    + Copy
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + std::hash::Hash
    + Display
    + std::fmt::Debug
    + Zero
    + Sub<Output = Self>
    + Send
    + Sync
    + 'static
{
    /// The function which converts a duration into a time interval, with remainder.
    /// Essentially, division with remainder.
    /// The Duration argument is the time since the last tick.
    fn division(duration: Duration) -> (Self, Duration);
}

/// Represents a bounded integer time interval.
/// Useful for modeling discrete time with few possibilities.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::Display,
    derive_more::Deref,
    derive_more::Add,
    derive_more::Sub,
    derive_more::From,
    derive_more::Into,
)]
pub struct FiniteTime<const N: usize, const T_MILLIS: u64>(UpTo<N>);

impl<const N: usize, const T_MILLIS: u64> exhaustive::Exhaustive for FiniteTime<N, T_MILLIS> {
    fn generate(u: &mut exhaustive::DataSourceTaker) -> exhaustive::Result<Self> {
        u.choice(N).map(|x| Self(UpTo::new(x)))
    }
}

// impl<const N: usize, const T_MILLIS: u64> From<UpTo<N>> for FiniteTime<N, T_MILLIS> {
//     fn from(t: UpTo<N>) -> Self {
//         Self(t)
//     }
// }

impl<const N: usize, const T_MILLIS: u64> Zero for FiniteTime<N, T_MILLIS> {
    fn zero() -> Self {
        Self(UpTo::new(0))
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl<const N: usize, const T_MILLIS: u64> TimeInterval for FiniteTime<N, T_MILLIS> {
    fn division(duration: Duration) -> (Self, Duration) {
        let (t, d) = int_time_scaling(N, Duration::from_millis(T_MILLIS))(duration);
        (Self(UpTo::new(t)), d)
    }
}

impl TimeInterval for RealTime {
    fn division(duration: Duration) -> (Self, Duration) {
        (Self(duration), Duration::ZERO)
    }
}

/// Wall clock time.
#[derive(
    // Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::Display,
    derive_more::Debug,
    derive_more::Deref,
    derive_more::Add,
    derive_more::Sub,
    derive_more::From,
    derive_more::Into,
)]
#[display("{}", _0.human_duration())]
#[debug("{}", _0.human_duration())]
pub struct RealTime(std::time::Duration);

impl Zero for RealTime {
    fn zero() -> Self {
        Self(std::time::Duration::ZERO)
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl Mul<usize> for RealTime {
    type Output = Self;
    fn mul(self, rhs: usize) -> Self {
        Self(self.0 * rhs as u32)
    }
}

/// [`TickBuffer`] is an essential component of any time-aware model.
/// (see https://en.wikipedia.org/wiki/Timed_automaton)
///
/// When using a model for both model checking as well as monitoring a real-world system,
/// you will often want to reprent time both discretely and continuously.
/// The discrete-time model will have only a few possible values for time intervals,
/// to minimize the combinatorial explosion of the state space.
/// The continuous-time model can track wall-clock time and match the real-world system.
///
/// The [`TickBuffer`] is a simple utility for working with both types of time.
/// The state transition (a "tick" event) which advances the clocks in your state machine needs to be emitted
/// by your system to represent real time passing in the real-world system. Whenever a tick
/// occurs, [`TickBuffer::tick`] should be called. Then, according to the needs of your model,
/// a sequence of time intervals will be returned, which represents the time that has passed since the last tick
/// event.
///
/// If using continuous time, one interval will be returned corresponding to the elapsed time.
/// If using discrete time, several intervals may be returned, or potentially zero intervals.
///
/// The tick function should be called by [`crate::mapping::ModelMapping`] *before* any other events are handled,
/// so that the model can know how much time has passed before handling any other actions.
pub struct TickBuffer<T: TimeInterval> {
    last_tick: Instant,
    phantom: PhantomData<T>,
}

impl<T: TimeInterval> TickBuffer<T> {
    /// Initialize the tick buffer with the current time.
    pub fn new(start: Instant) -> Self {
        Self {
            last_tick: start,
            phantom: PhantomData,
        }
    }

    /// Find out how much time has passed since the last tick, in terms of the time interval type
    /// appropriate to the model.
    pub fn tick(&mut self, now: Instant) -> impl Iterator<Item = T> {
        let mut elapsed = now - self.last_tick;
        let mut ticks = Vec::new();
        loop {
            let (t, d) = T::division(elapsed);
            elapsed = d;
            if t.is_zero() {
                break;
            }
            self.last_tick = now;
            ticks.push(t);
            if elapsed.is_zero() {
                break;
            }
        }

        ticks.into_iter()
    }
}

/// Helper function for expressing the division function for a bounded integer type
/// like [`UpTo<N>`](crate::id::UpTo).
/// `max_plus_1` would be `N`, and the `unit` is how much wall clock time corresponds
/// to a value of 1.
pub fn int_time_scaling(
    max_plus_1: usize,
    unit: Duration,
) -> impl Fn(Duration) -> (usize, Duration) {
    let unit = unit.as_micros() as u64;
    move |d| {
        let d = d.as_micros() as u64;
        if d.is_zero() {
            return (0, Duration::ZERO);
        }
        let q = d / unit;
        let t = q.min(max_plus_1 as u64 - 1);
        (t as usize, Duration::from_micros(d - t * unit))
    }
}

#[cfg(test)]
mod tests {

    use itertools::Itertools;

    use crate::id::UpTo;

    use super::*;

    #[test]
    fn test_time_scaling() {
        let sec = int_time_scaling(10, Duration::from_secs(1));
        assert_eq!(sec(Duration::from_secs(1)), (1, Duration::ZERO));
        assert_eq!(
            sec(Duration::from_millis(1500)),
            (1, Duration::from_millis(500))
        );
        assert_eq!(
            sec(Duration::from_millis(2500)),
            (2, Duration::from_millis(500))
        );
        assert_eq!(
            sec(Duration::from_millis(11111)),
            (9, Duration::from_millis(2111))
        );
    }

    #[test]
    fn test_tick_buffer_discrete() {
        let start = Instant::now();
        let d1 = Duration::from_millis(350);
        let d2 = Duration::from_millis(801);
        let d3 = Duration::from_millis(4000);
        let d4 = Duration::from_millis(5500);
        let d5 = Duration::from_millis(500);

        type T = FiniteTime<3, 1000>;

        let mut b = TickBuffer::<T>::new(start);

        assert_eq!(b.tick(start + d1).collect_vec(), vec![]);
        assert_eq!(
            b.tick(start + d1 + d2).collect_vec(),
            vec![UpTo::new(1).into()]
        );
        assert_eq!(
            b.tick(start + d1 + d2 + d3).collect_vec(),
            vec![UpTo::new(2).into(), UpTo::new(2).into()]
        );
        assert_eq!(
            b.tick(start + d1 + d2 + d3 + d4).collect_vec(),
            vec![
                UpTo::new(2).into(),
                UpTo::new(2).into(),
                UpTo::new(1).into()
            ]
        );
        assert_eq!(b.tick(start + d1 + d2 + d3 + d4 + d5).collect_vec(), vec![]);

        // Does not include d5
        assert_eq!(b.last_tick, start + d1 + d2 + d3 + d4);
    }

    #[test]
    fn test_tick_buffer_continuous() {
        let start = Instant::now();
        let d1 = Duration::from_millis(350);
        let d2 = Duration::from_millis(801);
        let mut b = TickBuffer::<RealTime>::new(start);

        assert_eq!(b.tick(start + d1).collect_vec(), vec![d1.into()]);
        assert_eq!(b.tick(start + d1 + d2).collect_vec(), vec![d2.into()]);
    }
}
