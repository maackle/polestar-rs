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
pub struct UpTo<const N: usize>(pub(super) usize);

impl<const N: usize> Id for UpTo<N> {
    fn choices() -> IdChoices {
        IdChoices::Small(N)
    }
}

impl<const N: usize> UpTo<N> {
    pub fn new(n: usize) -> Self {
        Self::try_from(n).expect(&format!("Attempted to initialize UpTo<{N}> with {n}"))
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

impl<const N: usize> exhaustive::Exhaustive for UpTo<N> {
    fn generate(u: &mut exhaustive::DataSourceTaker) -> exhaustive::Result<Self> {
        u.choice(N).map(Self)
    }
}

impl<const N: usize> TryFrom<usize> for UpTo<N> {
    type Error = String;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < N {
            Ok(Self(value))
        } else {
            Err(format!("Cannot use {value} for UpTo<{N}>"))
        }
    }
}

impl<const N: usize> std::fmt::Debug for UpTo<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

impl<const N: usize> std::fmt::Display for UpTo<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<const N: usize> std::ops::Add<usize> for UpTo<N> {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        Self((self.0 + rhs) % N)
    }
}

impl<const N: usize> std::ops::Sub<usize> for UpTo<N> {
    type Output = UpTo<N>;
    fn sub(self, rhs: usize) -> Self::Output {
        let rhs = rhs % N;
        let v = if self.0 < rhs {
            self.0 + N - rhs
        } else {
            self.0 - rhs
        };
        UpTo(v)
    }
}
