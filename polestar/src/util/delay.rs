use std::ops::{Add, Sub};

use num_traits::Zero;

use crate::id::Id;

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::Display,
    exhaustive::Exhaustive,
)]
pub enum Delay<T> {
    #[display("{}", _0)]
    Finite(T),
    #[display("∞")]
    Infinite,
}

impl<T> Delay<T>
where
    T: Id + Zero + Sub<usize, Output = T>,
    T::Error: std::fmt::Debug,
{
    pub fn finite(num: usize) -> Self {
        Delay::Finite(T::try_from(num).unwrap())
    }

    pub fn tick(self) -> Self {
        match self {
            Delay::Infinite => Delay::Infinite,
            Delay::Finite(delay) => Delay::Finite(if delay.is_zero() { delay } else { delay - 1 }),
        }
    }
}

impl<T> Add<Delay<T>> for Delay<T>
where
    T: Add<T, Output = T>,
{
    type Output = Self;
    fn add(self, rhs: Delay<T>) -> Self::Output {
        match (self, rhs) {
            (Delay::Infinite, _) => Delay::Infinite,
            (_, Delay::Infinite) => Delay::Infinite,
            (Delay::Finite(delay), Delay::Finite(rhs)) => Delay::Finite(delay + rhs),
        }
    }
}

impl<T> Zero for Delay<T>
where
    T: Zero + Add<T, Output = T>,
{
    fn zero() -> Self {
        Delay::Finite(T::zero())
    }

    fn is_zero(&self) -> bool {
        matches!(self, Delay::Finite(delay) if delay.is_zero())
    }
}
