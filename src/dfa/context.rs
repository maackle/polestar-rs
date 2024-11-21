use super::*;

/// Wrapper around an FSM which carries a context that gets injected into each event.
/// Useful for attaching some immutable context to the FSM which is not part of its own state.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Arbitrary, derive_more::Deref)]
pub struct Contextual<S, C> {
    #[deref]
    pub(super) fsm: S,
    // TODO: C: Clone
    pub context: Arc<C>,
}

impl<S, C> std::fmt::Debug for Contextual<S, C>
where
    S: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.fsm)
    }
}

impl<S, C> Contextual<S, C>
where
    S: Machine,
{
    pub fn new(fsm: S, context: C) -> Self {
        Self {
            fsm,
            context: Arc::new(context),
        }
    }
}

impl<S, C, E> Machine for Contextual<S, C>
where
    S: Machine<Action = (E, Arc<C>)>,
{
    type Action = E;
    type Fx = S::Fx;
    type Error = S::Error;

    fn transition(self, event: Self::Action) -> MachineResult<Self> {
        let context = self.context;
        let (fsm, fx) = Machine::transition(self.fsm, (event, context.clone()))?;
        Ok((Self { fsm, context }, fx))
    }

    fn is_terminal(&self) -> bool {
        self.fsm.is_terminal()
    }
}
