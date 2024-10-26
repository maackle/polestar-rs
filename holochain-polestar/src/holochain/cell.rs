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
    type Event = CellEvent;
    type Fx = ();

    fn transition(&mut self, _: Self::Event) -> Self::Fx {
        todo!()
    }
}

pub type CellFsm = CellState;
pub type CellActor = polestar::actor::ActorRead<CellFsm>;
