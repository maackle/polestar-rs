// mod inline_model_checker;
// pub use inline_model_checker::*;

use std::sync::Arc;

use parking_lot::Mutex;

use crate::{prelude::Projection, Machine};

pub trait EventHandler<Event, Error>: Send + Sync + 'static {
    fn handle(&self, event: Event) -> Result<(), Error>;
}

#[derive(Clone, derive_more::Constructor)]
pub struct EventSender<Event>(std::sync::mpsc::Sender<Event>);

impl<Event: Send + Sync + 'static> EventHandler<Event, anyhow::Error> for EventSender<Event> {
    fn handle(&self, event: Event) -> anyhow::Result<()> {
        self.0
            .send(event)
            .map_err(|e| anyhow::anyhow!("send event failed: {e:?}"))
    }
}

pub struct EventSink<Event, Error>(Mutex<Box<dyn EventHandler<Event, Error>>>);

impl<Event: Send + Sync + 'static, Error: Send + Sync + 'static> EventSink<Event, Error> {
    pub fn new(handler: impl EventHandler<Event, Error> + 'static) -> Self {
        Self(Mutex::new(Box::new(handler)))
    }

    pub fn handle(&self, event: Event) -> Result<(), Error> {
        self.0.lock().handle(event)
    }
}

#[derive(Debug)]
pub struct NullEventHandler;

impl<Event: Send + Sync + 'static, Error: Send + Sync + 'static> EventHandler<Event, Error>
    for NullEventHandler
{
    fn handle(&self, _event: Event) -> Result<(), Error> {
        Ok(())
    }
}
