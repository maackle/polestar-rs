use super::*;

/// Wrapper around an FSM which carries a context that gets injected into each event.
/// Useful for attaching some immutable context to the FSM which is not part of its own state.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Arbitrary)]
pub struct FsmContext<S: Fsm, C> {
    pub(super) fsm: S,
    // TODO: C: Clone
    pub(super) context: Arc<C>,
}

impl<S, C> std::fmt::Debug for FsmContext<S, C>
where
    S: Fsm + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.fsm)
    }
}

impl<S, C> FsmContext<S, C>
where
    S: Fsm,
{
    pub fn new(fsm: S, context: C) -> Self {
        Self {
            fsm,
            context: Arc::new(context),
        }
    }
}

impl<S, C, E> Fsm for FsmContext<S, C>
where
    S: Fsm<Event = (E, Arc<C>)>,
{
    type Event = E;
    type Fx = S::Fx;
    type Error = S::Error;

    fn transition(self, event: Self::Event) -> FsmResult<Self> {
        let context = self.context;
        let (fsm, fx) = Fsm::transition(self.fsm, (event, context.clone()))?;
        Ok((Self { fsm, context }, fx))
    }

    fn is_terminal(&self) -> bool {
        self.fsm.is_terminal()
    }
}
