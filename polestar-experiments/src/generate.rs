//! Helpers for generating arbitrary values, used in probabilistic model checking

use prop::strategy::ValueTree;
use proptest::prelude::*;

/// An interface for the arbitrary generation of values.
/// This is used for stochastic model checking (as opposed to exhaustive model checking).
/// Currently, the only implementation uses proptest's Arbitrary trait.
pub trait Generator {
    /// Generate a single value
    fn generate<T: Arbitrary>(&mut self) -> Result<T, prop::test_runner::Reason> {
        self.generate_with(T::arbitrary())
    }

    /// Generate a single value with a custom strategy
    fn generate_with<T: Arbitrary>(
        &mut self,
        strategy: impl Strategy<Value = T>,
    ) -> Result<T, prop::test_runner::Reason>;
}

impl Generator for prop::test_runner::TestRunner {
    fn generate_with<T: Arbitrary>(
        &mut self,
        strategy: impl Strategy<Value = T>,
    ) -> Result<T, prop::test_runner::Reason> {
        Ok(strategy.new_tree(self)?.current())
    }
}
