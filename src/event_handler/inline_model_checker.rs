use std::sync::Arc;

use parking_lot::Mutex;

use crate::{prelude::*, projection::ErrorOf};

use super::EventHandler;

pub struct InlineModelChecker<P: Projection> {
    model: Option<P::Model>,
    projection: Arc<Mutex<P>>,
    config: InlineModelCheckerConfig,
}

#[derive(Default)]
pub struct InlineModelCheckerConfig {
    require_event_mapping: bool,
}

#[derive(Debug, derive_more::Error, derive_more::From)]
pub enum InlineModelCheckerError<E> {
    #[from]
    Model(E),
    NoEventMapping,
    NoModel,
}

impl<P> InlineModelChecker<P>
where
    P: Projection,
{
    pub fn new(
        projection: Arc<Mutex<P>>,
        initial: &P::System,
        config: InlineModelCheckerConfig,
    ) -> Option<Self> {
        let model = Some(projection.lock().map_state(initial)?);
        Some(Self {
            model,
            projection,
            config,
        })
    }
}

impl<P> EventHandler<P::Event, InlineModelCheckerError<ErrorOf<P::Model>>> for InlineModelChecker<P>
where
    P: Projection,
{
    fn handle(&self, event: P::Event) -> Result<(), InlineModelCheckerError<ErrorOf<P::Model>>> {
        let mut projection = self.projection.lock();
        if let Some(action) = projection.map_event(event) {
            let (next, _fx) = self
                .model
                .take()
                .ok_or(InlineModelCheckerError::NoModel)?
                .transition(action)?;
            self.model = Some(next);
        } else if self.config.require_event_mapping {
            return Err(InlineModelCheckerError::NoEventMapping);
        }
        Ok(())
    }
}
