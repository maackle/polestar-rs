use super::*;

/// A state machine where the machine itself has no data.
///
/// This simplifies implementations where there is no configuration
pub trait MachineUnit
where
    Self: Cog + Send + Sync + 'static,
{
    /// The type representing the actions (transitions) of the machine
    type Action: Cog;
    /// The type representing the side effects of the machine
    type Fx: Cog;
    /// The type representing the errors that can occur during transitions
    type Error: Debug + Send + Sync;

    /// Defines the transition function of the machine in terms of state and action only.
    fn transition(self, action: Self::Action) -> Result<(Self, Self::Fx), Self::Error>;
}

/// The [Machine] implementation in terms of a [MachineUnit]
pub struct StateModel<U: MachineUnit>(PhantomData<U>);

impl<U> Machine for StateModel<U>
where
    U: MachineUnit,
{
    type State = U;
    type Action = U::Action;
    type Fx = U::Fx;
    type Error = U::Error;

    fn transition(
        &self,
        state: Self::State,
        action: Self::Action,
    ) -> Result<(Self::State, Self::Fx), Self::Error> {
        U::transition(state, action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_model() {
        #[derive(Debug, Clone)]
        struct Adder {
            sum: u32,
        }

        impl MachineUnit for Adder {
            type Action = u32;
            type Fx = ();
            type Error = anyhow::Error;

            fn transition(self, action: Self::Action) -> Result<(Self, Self::Fx), Self::Error> {
                Ok((
                    Self {
                        sum: self.sum + action,
                    },
                    (),
                ))
            }
        }
    }
}
