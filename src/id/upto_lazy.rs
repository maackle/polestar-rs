use super::*;

use once_cell::sync::Lazy;
use parking_lot::Mutex;

static MAP: Lazy<Mutex<HashMap<u64, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));

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
pub struct UpToLazy<const UID: u64>(usize);

impl<const UID: u64> Id for UpToLazy<UID> {
    fn choices() -> IdChoices {
        IdChoices::Small(Self::limit())
    }
}

impl<const UID: u64> UpToLazy<UID> {
    pub fn set_limit(n: usize) {
        let mut map = MAP.lock();
        let existing = map.insert(UID, n);
        assert!(existing.is_none(), "Attempted to set limit for {UID} twice");
    }

    pub fn limit() -> usize {
        let map = MAP.lock();
        *map.get(&UID)
            .unwrap_or_else(|| panic!("No limit set for {UID}"))
    }

    pub fn new(n: usize) -> Self {
        Self::try_from(n).expect(&format!("Attempted to initialize UpToLazy<{UID}> with {n}"))
    }

    pub fn modulo(n: usize) -> Self {
        Self(n % Self::limit())
    }

    pub fn all_values() -> Vec<Self> {
        (0..Self::limit())
            .map(Self::new)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

impl<const UID: u64> exhaustive::Exhaustive for UpToLazy<UID> {
    fn generate(u: &mut exhaustive::DataSourceTaker) -> exhaustive::Result<Self> {
        u.choice(Self::limit()).map(Self)
    }
}

impl<const UID: u64> TryFrom<usize> for UpToLazy<UID> {
    type Error = String;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < Self::limit() {
            Ok(Self(value))
        } else {
            Err(format!("Cannot use {value} for UpToLazy<{UID}>"))
        }
    }
}

impl<const UID: u64> std::fmt::Debug for UpToLazy<UID> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

impl<const UID: u64> std::fmt::Display for UpToLazy<UID> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<const UID: u64> std::ops::Add<usize> for UpToLazy<UID> {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        Self((self.0 + rhs) % Self::limit())
    }
}

impl<const UID: u64> std::ops::Sub<usize> for UpToLazy<UID> {
    type Output = UpToLazy<UID>;
    fn sub(self, rhs: usize) -> Self::Output {
        let rhs = rhs % Self::limit();
        let v = if self.0 < rhs {
            self.0 + Self::limit() - rhs
        } else {
            self.0 - rhs
        };
        UpToLazy(v)
    }
}
