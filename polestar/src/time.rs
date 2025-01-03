use num_traits::*;
use std::{
    fmt::Display,
    ops::{Mul, Sub},
    time::{Duration, Instant},
};

/// Types which can represent an interval of time as needed by a model.
pub trait TimeInterval:
    Clone
    + Copy
    + PartialEq
    + Eq
    + std::hash::Hash
    + Display
    + std::fmt::Debug
    + Zero
    + Sub<Output = Self>
    + Mul<usize, Output = Self>
{
}

impl TimeInterval for usize {}
impl<const N: usize> TimeInterval for crate::id::UpTo<N> {}
impl TimeInterval for RealTime {}

/// Wall clock time.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
    derive_more::Deref,
    derive_more::Add,
    derive_more::Sub,
    derive_more::From,
    derive_more::Into,
)]
#[display("{:?}", _0)]
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

pub struct TickBuffer<T: TimeInterval> {
    last_tick: Instant,
    /// The function which converts a duration into a time interval, with remainder.
    /// Essentially, division with remainder.
    /// The Duration argument is the time since the last tick.
    division: Box<dyn Fn(Duration) -> (T, Duration)>,
}

impl<T: TimeInterval> TickBuffer<T> {
    pub fn new(start: Instant, scaling: impl Fn(Duration) -> (T, Duration) + 'static) -> Self {
        Self {
            last_tick: start,
            division: Box::new(scaling),
        }
    }

    pub fn tick(&mut self, now: Instant) -> impl Iterator<Item = T> {
        let mut elapsed = now - self.last_tick;
        let mut ticks = Vec::new();
        loop {
            let (t, d) = (self.division)(elapsed);
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
        let q = (d / unit) as u64;
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

        let mut b = TickBuffer::<UpTo<3>>::new(start.clone(), |d| {
            let (t, q) = int_time_scaling(3, Duration::from_secs(1))(d);
            (UpTo::new(t), q)
        });

        assert_eq!(b.tick(start + d1).collect_vec(), vec![]);
        assert_eq!(b.tick(start + d1 + d2).collect_vec(), vec![UpTo::new(1)]);
        assert_eq!(
            b.tick(start + d1 + d2 + d3).collect_vec(),
            vec![UpTo::new(2), UpTo::new(2)]
        );
        assert_eq!(
            b.tick(start + d1 + d2 + d3 + d4).collect_vec(),
            vec![UpTo::new(2), UpTo::new(2), UpTo::new(1)]
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
        let mut b = TickBuffer::<RealTime>::new(start, |d| (d.into(), Duration::ZERO));

        assert_eq!(b.tick(start + d1).collect_vec(), vec![d1.into()]);
        assert_eq!(b.tick(start + d1 + d2).collect_vec(), vec![d2.into()]);
    }
}
