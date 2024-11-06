use polestar::prelude::*;

use super::{model, system, NodeId};

impl Projection<model::NetworkOp> for system::Panopticon {
    type Event = (NodeId, system::NodeEvent);

    fn apply(&mut self, (id, event): Self::Event) {
        self.get_mut(&id).unwrap().handle_event(event);
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
