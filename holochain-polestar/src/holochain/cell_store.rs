use std::{collections::HashMap, convert::Infallible};

use polestar::prelude::*;

use super::*;

#[derive(Default)]
pub struct CellStore(FsmHashMap<CellId, CellFsm>);

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
        let () = match e {
            CellStoreEvent::CellEvent(id, e) => {
                let () = self
                    .0
                    .transition_mut(id.clone(), e)
                    .ok_or(anyhow!("cell not found: {id:?}"))??;
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
