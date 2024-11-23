use crate::util::first;

trait Nfm
where
    Self: Sized,
{
    type Action: Clone;
    type Fx: Clone;
    type Error: std::fmt::Debug;

    fn transition(self, event: Self::Action) -> Vec<(Self, Vec<Self::Fx>)>;

    /// Perform a transition and ignore the effect, when the effect is `()`.
    fn transition_(self, event: Self::Action) -> Vec<Self>
    where
        Self: Nfm<Fx = ()>,
    {
        self.transition(event).into_iter().map(first).collect()
    }

    fn apply_actions(
        self,
        actions: impl IntoIterator<Item = Self::Action>,
    ) -> Vec<(Self, Vec<Self::Fx>)> {
        let mut states = vec![(self, vec![])];
        for action in actions.into_iter() {
            states = states
                .into_iter()
                .flat_map(|(state, fx)| {
                    let fx = fx.clone();
                    state
                        .transition(action.clone())
                        .into_iter()
                        .map(move |(state2, fx2)| {
                            let mut fx = fx.clone();
                            fx.extend(fx2);
                            (state2, fx)
                        })
                })
                .collect();
        }
        states
    }

    fn apply_actions_(self, actions: impl IntoIterator<Item = Self::Action>) -> Vec<Self> {
        self.apply_actions(actions).into_iter().map(first).collect()
    }

    /// Designates this state as a terminal state.
    ///
    /// This is an optional hint, useful for generating diagrams from FSMs.
    fn is_terminal(&self) -> bool {
        false
    }
}

type NfmResult<S> = Result<Vec<(S, <S as Nfm>::Fx)>, <S as Nfm>::Error>;
