use std::ops::Mul;

use num_traits::{Bounded, WrappingAdd, WrappingSub};

use super::*;

/// A number in the range [0, N)
#[derive(
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::Into,
    derive_more::Deref,
)]
#[cfg_attr(feature = "recording", derive(serde::Serialize, serde::Deserialize))]
pub struct UpTo<const N: usize, const WRAP: bool = false>(pub(super) usize);

impl<const N: usize, const WRAP: bool> Id for UpTo<N, WRAP> {
    fn choices() -> IdChoices {
        IdChoices::Small(N)
    }
}

impl<const N: usize, const WRAP: bool> UpTo<N, WRAP> {
    pub fn new(n: usize) -> Self {
        Self::try_from(n).expect(&format!("Attempted to initialize UpTo<{N}> with {n}"))
    }

    pub fn limit() -> usize {
        N
    }

    pub fn modulo(n: usize) -> Self {
        Self(n % N)
    }

    pub fn all_values() -> [Self; N] {
        (0..N)
            .map(Self::new)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

impl<const N: usize, const WRAP: bool> exhaustive::Exhaustive for UpTo<N, WRAP> {
    fn generate(u: &mut exhaustive::DataSourceTaker) -> exhaustive::Result<Self> {
        u.choice(N).map(Self)
    }
}

impl<const N: usize, const WRAP: bool> TryFrom<usize> for UpTo<N, WRAP> {
    type Error = String;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < N {
            Ok(Self(value))
        } else {
            Err(format!("Cannot use {value} for UpTo<{N}>"))
        }
    }
}

impl<const N: usize, const WRAP: bool> std::fmt::Debug for UpTo<N, WRAP> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

impl<const N: usize, const WRAP: bool> std::fmt::Display for UpTo<N, WRAP> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<const N: usize, const WRAP: bool> std::ops::Add<usize> for UpTo<N, WRAP> {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        if WRAP {
            Self((self.0 + rhs) % N)
        } else {
            Self(self.0 + rhs)
        }
    }
}

impl<const N: usize, const WRAP: bool> std::ops::Add<UpTo<N, WRAP>> for UpTo<N, WRAP> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        self + rhs.0
    }
}

// impl<const N: usize> WrappingAdd for UpTo<N> {
//     fn wrapping_add(&self, rhs: &Self) -> Self {
//         Self((self.0 + rhs.0) % N)
//     }
// }

impl<const N: usize, const WRAP: bool> std::ops::Sub<usize> for UpTo<N, WRAP> {
    type Output = UpTo<N, WRAP>;
    fn sub(self, rhs: usize) -> Self::Output {
        if WRAP {
            let v = if self.0 < rhs {
                self.0 + N - rhs
            } else {
                self.0 - rhs
            };
            Self(v)
        } else {
            Self(self.0 - rhs)
        }
    }
}

impl<const N: usize, const WRAP: bool> std::ops::Sub<UpTo<N, WRAP>> for UpTo<N, WRAP> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self - rhs.0
    }
}

// impl<const N: usize> WrappingSub for UpTo<N> {
//     fn wrapping_sub(&self, rhs: &Self) -> Self {
//         let rhs = rhs.0;
//         let v = if self.0 < rhs {
//             self.0 + N - rhs
//         } else {
//             self.0 - rhs
//         };
//         UpTo(v)
//     }
// }

impl<const N: usize> num_traits::Zero for UpTo<N> {
    fn zero() -> Self {
        Self(0)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl<const N: usize> Mul<UpTo<N>> for UpTo<N> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.0 * rhs.0)
    }
}

impl<const N: usize> Mul<usize> for UpTo<N> {
    type Output = Self;
    fn mul(self, rhs: usize) -> Self::Output {
        Self::new(self.0 * rhs)
    }
}

impl<const N: usize> Bounded for UpTo<N> {
    fn min_value() -> Self {
        Self(0)
    }

    fn max_value() -> Self {
        Self(N - 1)
    }
}
