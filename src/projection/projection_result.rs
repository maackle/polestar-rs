use std::marker::PhantomData;

use super::*;

/// Lift a projection to a result type.
/// If P is projection from system S into a model M,
/// then this is a projection from Result<S, E> into M.
pub struct ProjectionResult<P, E> {
    projection: P,
    phantom: PhantomData<E>,
}

impl<Proj, Model, Error> Projection<Model> for ProjectionResult<Proj, Error>
where
    Model: Fsm,
    Proj: Projection<Model>,
{
    type System = Result<Proj::System, Error>;
    type Event = Proj::Event;

    fn apply(&self, state: &mut Self::System, event: Self::Event) {
        if let Ok(s) = state {
            Proj::apply(&self.projection, s, event)
        }
    }

    fn map_state(&self, system: &Self::System) -> Option<Model> {
        match system {
            Ok(s) => Proj::map_state(&self.projection, s),
            _ => None,
        }
    }

    fn map_event(&self, event: Self::Event) -> Option<<Model as Fsm>::Action> {
        Proj::map_event(&self.projection, event)
    }

    fn gen_state(&self, generator: &mut impl Generator, state: Model) -> Self::System {
        Ok(Proj::gen_state(&self.projection, generator, state))
    }

    fn gen_event(
        &self,
        generator: &mut impl Generator,
        event: <Model as Fsm>::Action,
    ) -> Self::Event {
        Proj::gen_event(&self.projection, generator, event)
    }
}
