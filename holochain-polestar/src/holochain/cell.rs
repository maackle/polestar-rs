use crate::*;

pub(crate) enum CellEvent {
    // CallZome(CallZome),
}

pub(crate) enum CellState {}

pub type CellFsm = ParamFsm<CellState, CellEvent, ()>;
