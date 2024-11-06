use polestar::prelude::*;

use super::*;

pub struct Nodes(Vec<Node>);

impl Projection<NetworkOp> for Nodes {
    type Event = !;

    fn apply(&mut self, event: Self::Event) {
        todo!()
    }

    fn map_state(&self) -> Option<NetworkOp> {
        todo!()
    }

    fn map_event(&self, event: Self::Event) -> Option<<NetworkOp as Fsm>::Event> {
        todo!()
    }

    fn gen_state(&self, generator: &mut impl Generator, state: NetworkOp) -> Self {
        todo!()
    }

    fn gen_event(
        &self,
        generator: &mut impl Generator,
        event: <NetworkOp as Fsm>::Event,
    ) -> Self::Event {
        todo!()
    }
}
