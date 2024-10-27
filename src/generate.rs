use prop::strategy::ValueTree;
use proptest::prelude::*;

pub trait Generate<T> {
    // TODO: like From, but includes a Generator to fill in the extra info
}

pub trait Generator {
    fn generate<T: Arbitrary>(&mut self) -> Result<T, prop::test_runner::Reason> {
        self.generate_with(T::arbitrary())
    }

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
