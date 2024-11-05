use std::{collections::HashMap, convert::Infallible};

use polestar::fsm::FsmResult;

use super::*;

#[derive(Default)]
pub struct CellStore(HashMap<CellId, CellFsm>);

pub enum CellStoreEvent {
    CellEvent(CellId, CellEvent),
    AddCell(CellId),
    RemoveCell(CellId),
}

#[must_use]
pub struct CellStoreFx;

impl polestar::Fsm for CellStore {
    type Event = CellStoreEvent;
    type Fx = CellStoreFx;
    type Error = anyhow::Error;

    fn transition(mut self, e: Self::Event) -> FsmResult<Self> {
        let _fx = match e {
            CellStoreEvent::CellEvent(id, e) => {
                let (state, fx) = self
                    .0
                    .remove(&id)
                    .ok_or(anyhow!("cell not found: {id:?}"))?
                    .transition(e)?;
                self.0.insert(id, state);
                fx
            }
            CellStoreEvent::AddCell(id) => {
                self.0.insert(id, CellFsm::default());
            }
            CellStoreEvent::RemoveCell(id) => {
                self.0.remove(&id);
            }
        };
        Ok((self, CellStoreFx))
    }
}
