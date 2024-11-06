use polestar::prelude::*;

use super::{model, system};

impl Projection<model::NetworkOp> for system::Nodes {
    type Event = model::NetworkOpEvent;

    fn apply(&mut self, event: Self::Event) {
        todo!()
    }

    fn map_state(&self) -> Option<model::NetworkOp> {
        todo!()
    }

    fn map_event(&self, event: Self::Event) -> Option<model::NetworkOpEvent> {
        todo!()
    }

    fn gen_state(&self, generator: &mut impl Generator, state: model::NetworkOp) -> Self {
        todo!()
    }

    fn gen_event(
        &self,
        generator: &mut impl Generator,
        event: model::NetworkOpEvent,
    ) -> Self::Event {
        todo!()
    }
}
