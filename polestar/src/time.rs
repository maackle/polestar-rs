use num_traits::*;
use std::{
    fmt::Display,
    ops::{Mul, Sub},
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
    + Bounded
{
}
impl TimeInterval for usize {}

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

impl Bounded for RealTime {
    fn min_value() -> Self {
        Self(std::time::Duration::ZERO)
    }

    fn max_value() -> Self {
        Self(std::time::Duration::MAX)
    }
}

impl TimeInterval for RealTime {}
impl<const N: usize> TimeInterval for crate::id::UpTo<N> {}
