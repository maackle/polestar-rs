use std::collections::HashMap;

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
    type Fx = anyhow::Result<CellStoreFx>;

    fn transition(&mut self, e: Self::Event) -> Self::Fx {
        match e {
            CellStoreEvent::CellEvent(id, e) => {
                self.0
                    .get_mut(&id)
                    .ok_or(anyhow!("cell not found: {id:?}"))?
                    .transition(e);
            }
            CellStoreEvent::AddCell(id) => {
                self.0.insert(id, CellFsm::default());
            }
            CellStoreEvent::RemoveCell(id) => {
                self.0.remove(&id);
            }
        }
        Ok(CellStoreFx)
    }
}
