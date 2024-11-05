use std::{collections::HashMap, convert::Infallible};

use polestar::fsm::FsmResult;

use super::*;

#[derive(Default)]
pub struct AppStore(HashMap<AppId, AppFsm>);

pub enum AppStoreEvent {
    AppEvent(AppId, AppEvent),
    InstallApp(AppId, AppContext),
    RemoveApp(AppId),
}

#[must_use]
#[derive(Debug, derive_more::From)]
pub enum AppStoreFx {
    AppFx(AppId, AppFx),
}

impl polestar::Fsm for AppStore {
    type Event = AppStoreEvent;
    type Fx = Option<AppStoreFx>;
    type Error = anyhow::Error;

    fn transition(mut self, e: Self::Event) -> FsmResult<Self> {
        let fx = match e {
            AppStoreEvent::AppEvent(id, e) => {
                let (state, fx) = self
                    .0
                    .remove(&id)
                    .ok_or(anyhow!("app not found: {id}"))?
                    .transition(e)?;
                self.0.insert(id.clone(), state);
                fx.map(|fx| (id, fx).into())
            }
            AppStoreEvent::InstallApp(id, context) => {
                self.0.insert(id, AppFsm::new(AppState::default(), context));
                None
            }
            AppStoreEvent::RemoveApp(id) => {
                self.0.remove(&id);
                None
            }
        };
        Ok((self, fx))
    }
}
