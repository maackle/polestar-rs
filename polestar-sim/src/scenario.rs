use polestar_core::{ActionOf, Behavior, FxOf, StateMachine, StateOf};

/// A Scenario takes an initial model State and a Behavior,
/// and uses the Behavior to produce Actions to perform state transitions.
pub struct Scenario<B>
where
    B: Behavior,
{
    state: StateMachine<B::Model>,
    initial_state: StateOf<B::Model>,
    behavior: B,
    steps: usize,
    handlers: ScenarioHandlers<B>,
}

struct ScenarioHandlers<B>
where
    B: Behavior,
{
    on_step: Option<Box<dyn Fn(&Scenario<B>) + Send + Sync + 'static>>,
    on_action: Option<(
        String,
        Box<dyn Fn(&ActionOf<B::Model>) + Send + Sync + 'static>,
    )>,
    on_fx: Option<(String, Box<dyn Fn(&FxOf<B::Model>) + Send + Sync + 'static>)>,
}

impl<B> Default for ScenarioHandlers<B>
where
    B: Behavior,
{
    fn default() -> Self {
        Self {
            on_step: None,
            on_action: None,
            on_fx: None,
        }
    }
}

impl<B> Scenario<B>
where
    B: Behavior,
    FxOf<B::Model>: std::fmt::Debug,
{
    /// Create a new Scenario with the given Model, initial state, and Behavior.
    pub fn initial(state: StateMachine<B::Model>, behavior: B) -> Self {
        let initial_state = state.state().clone();
        Self {
            state,
            initial_state,
            behavior,
            steps: 0,

            handlers: ScenarioHandlers::default(),
        }
    }

    /// Bring the Scenario back to its initial state.
    pub fn reset(&mut self) {
        self.state.set_state(self.initial_state.clone());
        self.steps = 0;
    }

    /// Add a handler for each step of the Scenario.
    pub fn on_step(mut self, on_step: impl Fn(&Self) + Send + Sync + 'static) -> Self {
        self.handlers.on_step = Some(Box::new(on_step));
        self
    }

    /// Add a handler for each Action performed by the Scenario.
    pub fn on_action(
        mut self,
        line: &str,
        on_action: impl Fn(&ActionOf<B::Model>) + Send + Sync + 'static,
    ) -> Self {
        self.handlers.on_action = Some((line.to_string(), Box::new(on_action)));
        self
    }

    /// Add a handler for each Fx produced by the Scenario.
    pub fn on_fx(
        mut self,
        line: &str,
        on_fx: impl Fn(&FxOf<B::Model>) + Send + Sync + 'static,
    ) -> Self {
        self.handlers.on_fx = Some((line.to_string(), Box::new(on_fx)));
        self
    }

    /// Advance the Scenario by one step.
    pub fn advance(&mut self) -> anyhow::Result<Vec<ActionOf<B::Model>>> {
        if let Some(on_step) = self.handlers.on_step.as_ref() {
            on_step(self);
        }
        let actions = self.behavior.next_tick(&self.state.state())?;
        for action in actions.iter() {
            let fx = self.state.step(action.clone())?;
            if let Some((_, on_action)) = self.handlers.on_action.as_ref() {
                on_action(action);
            }
            if let Some((_, on_fx)) = self.handlers.on_fx.as_ref() {
                on_fx(&fx);
            }
            let unhandled = self.behavior.handle_fx(&self.state.state(), fx)?;
            if let Some(fx) = unhandled {
                tracing::warn!("Scenario received unhandled effects: {:?}", fx);
            }
        }
        self.steps += 1;
        Ok(actions)
    }

    /// Get the current state of the Scenario.
    pub fn state(&self) -> &StateOf<B::Model> {
        self.state.state()
    }

    /// Get the number of steps taken by the Scenario so far.
    pub fn steps(&self) -> usize {
        self.steps
    }
}
