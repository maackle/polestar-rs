use std::convert::Infallible;

use polestar::fsm::FsmResult;

use crate::*;

pub enum CellEvent {
    // CallZome(CallZome),
}

#[derive(Default)]
pub enum CellState {
    #[default]
    Uninit,
}

impl polestar::Fsm for CellState {
    type Action = CellEvent;
    type Fx = ();
    type Error = Infallible;

    fn transition(mut self, _: Self::Action) -> FsmResult<Self> {
        todo!()
    }
}

pub type CellFsm = CellState;
pub type CellActor = polestar::actor::ActorRead<CellFsm>;
