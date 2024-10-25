use std::collections::HashMap;

use super::*;

#[derive(Default)]
pub struct AppStore(HashMap<AppId, AppFsm>);

pub enum AppStoreEvent {
    AppEvent(AppId, AppEvent),
    InstallApp(AppId, AppContext),
    RemoveApp(AppId),
}

#[must_use]
pub struct AppStoreFx;

impl polestar::Fsm for AppStore {
    type Event = AppStoreEvent;
    type Fx = anyhow::Result<AppStoreFx>;

    fn transition(&mut self, e: Self::Event) -> Self::Fx {
        match e {
            AppStoreEvent::AppEvent(id, e) => {
                self.0
                    .get_mut(&id)
                    .ok_or(anyhow!("app not found: {id}"))?
                    .transition(e);
            }
            AppStoreEvent::InstallApp(id, context) => {
                self.0.insert(id, AppFsm::new(AppState::default(), context));
            }
            AppStoreEvent::RemoveApp(id) => {
                self.0.remove(&id);
            }
        }
        Ok(AppStoreFx)
    }
}
