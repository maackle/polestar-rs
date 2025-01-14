//! Process events emitted by a system, usually eventually passed on
//! to a [`polestar::mapping::ModelMapping`] to hook up a system to its model.

// mod inline_model_checker;
// pub use inline_model_checker::*;

/// A type which can handle emitted events
pub trait EventHandler<Event>: Send + Sync + 'static {
    /// Any errors in handling events will produce this error type
    type Error: Send + Sync + 'static;

    /// Handle one event
    fn handle(&mut self, event: &Event) -> Result<(), Self::Error>;
}

/// One simple way to handle events is to send them to a channel receiver
#[derive(Clone, derive_more::Constructor)]
pub struct EventSender<Event>(std::sync::mpsc::Sender<Event>);

impl<Event: Clone + Send + Sync + 'static> EventHandler<Event> for EventSender<Event> {
    type Error = anyhow::Error;

    fn handle(&mut self, event: &Event) -> anyhow::Result<()> {
        self.0
            .send(event.clone())
            .map_err(|e| anyhow::anyhow!("send event failed: {e:?}"))
    }
}

#[cfg(feature = "nonessential")]
/// A general event handler, using a mutex to ensure thread safety
pub struct EventSink<Event, Error>(parking_lot::Mutex<Box<dyn EventHandler<Event, Error = Error>>>);

#[cfg(feature = "nonessential")]
impl<Event: Send + Sync + 'static, Error: Send + Sync + 'static> EventSink<Event, Error> {
    pub fn new(handler: impl EventHandler<Event, Error = Error> + 'static) -> Self {
        Self(parking_lot::Mutex::new(Box::new(handler)))
    }

    pub fn handle(&mut self, event: &Event) -> Result<(), Error> {
        self.0.lock().handle(event)
    }
}

/// An event mishandler, which does nothing
#[derive(Debug)]
pub struct NullEventHandler;

impl<Event: Send + Sync + 'static> EventHandler<Event> for NullEventHandler {
    type Error = anyhow::Error;

    fn handle(&mut self, _event: &Event) -> Result<(), Self::Error> {
        Ok(())
    }
}
