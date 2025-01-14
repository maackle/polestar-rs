//! Connect a real-world system to a model of that system.

use core::fmt::Debug;
use std::{fs, io, io::Write, path::PathBuf};

use itertools::Itertools;
use serde::{de::DeserializeOwned, Serialize};

use crate::prelude::*;

/// Maps a model to the system of which it is a model.
///
/// # Invariants:
///
/// In order to be a valid mapping, some invariants must hold:
///
/// **Idempotency**: The ModelMapping is allowed to mutate itself when mapping states
/// and events. This mutation must be idempotent, such that repeated calls to either function
/// using the same input will always yield the same output.
///
/// **Commutativity**: The state transitions of the system must commute
/// with the model's transitions according to the following diagram,
/// where:
/// - `map_state` and `map_event` are the functions defined by the ModelMapping trait
/// - `S` and `S'` are the previous and next states of the system,
/// - `Q` and `Q'` are the previous and next states of the model,
/// - `event` causes a transition of the system state
/// - `action` causes a transition of the model state
///
/// ```text
///       S ─────transition(event)───────> S'
///       │                   ┆            │
///       │                   ┆            │
///    map_state         map_event     map_state
///       │                   ┆            │
///       V                   V            V
///       Q ─────transition(action)──────> Q'
/// ```
/// In other words, transitioning the system state `S` to `S'` and then
/// using `map_state` to obtain a model state `Q'`, but yield the same result
/// as first using `map_state` to go from system state `S` to model state `Q`,
/// and then using the model's transition function to obtain `Q'`.
///
/// Upholding this invariant ensures that the model is properly tracking the system state
/// through its transitions.
///
pub trait ModelMapping
where
    Self::Model: Machine,
{
    /// The model being mapped.
    ///
    /// (Model must implement [`Machine`], and so has associated types for `State` and `Action`)
    type Model;

    /// The state of the system which the model is a model of.
    type System;

    /// The events which the system emits to represent its own state changes.
    type Event;

    /// Map a system state to a model State.
    fn map_state(&mut self, system: &Self::System) -> Option<StateOf<Self::Model>>;

    /// Map a system event to a model Action.
    fn map_event(&mut self, event: &Self::Event) -> Vec<ActionOf<Self::Model>>;
}

/// Helper type for accessing the associated State type of a Machine.
pub type StateOf<M> = <M as Machine>::State;

/// Helper type for accessing the associated Action type of a Machine.
pub type ActionOf<M> = <M as Machine>::Action;

/// Helper type for accessing the associated Error type of a Machine.
pub type ErrorOf<M> = <M as Machine>::Error;

/// One way to record actions from the system is by simply writing their JSON
/// representation to a file, to be read back later.
#[cfg(feature = "recording")]
pub struct JsonActionWriter<M: ModelMapping> {
    mapping: M,
    path: PathBuf,
}

impl<M: ModelMapping> JsonActionWriter<M>
where
    M::Event: Debug,
    ActionOf<M::Model>: Serialize,
{
    pub fn new(path: impl Into<PathBuf>, mapping: M) -> io::Result<Self> {
        let path = path.into();
        fs::File::create(&path)?;
        Ok(Self { mapping, path })
    }

    pub fn write_line_raw(&mut self, what: &str) -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(false)
            .append(true)
            .open(&self.path)?;
        file.write_all(what.as_bytes())?;
        file.write_all("\n".as_bytes())?;
        Ok(())
    }

    pub fn write_event(&mut self, event: &M::Event) -> io::Result<()> {
        let actions = self.mapping.map_event(event);
        if actions.is_empty() {
            tracing::warn!("no action for event: {event:?}");
        } else {
            for action in actions {
                self.write_action(action)?;
            }
        }

        Ok(())
    }

    fn write_action(&mut self, action: ActionOf<M::Model>) -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(false)
            .append(true)
            .open(&self.path)?;

        let mut json = serde_json::to_string(&action)?;
        json.push('\n');
        file.write_all(json.as_bytes())?;

        Ok(())
    }
}

impl<M: ModelMapping> crate::EventHandler<M::Event> for JsonActionWriter<M>
where
    M: Send + Sync + 'static,
    M::Event: Debug + Send + Sync,
    ActionOf<M::Model>: Serialize,
{
    type Error = io::Error;

    fn handle(&mut self, event: &M::Event) -> Result<(), Self::Error> {
        self.write_event(event)
    }
}

/// Read actions written via JsonActionWriter into a Vec of model actions
pub fn read_actions_from_json_file<M: Machine>(
    path: impl Into<PathBuf>,
) -> serde_json::Result<Vec<ActionOf<M>>>
where
    ActionOf<M>: DeserializeOwned,
{
    let path = path.into();
    let text = std::fs::read_to_string(path).unwrap();
    assert!(!text.is_empty(), "events file is empty");
    let text = text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .join(",");
    let json = format!("[{}]", text);
    serde_json::from_str(&json)
}

// #[derive(Default)]
// pub struct EventHandlers<Event, Error> {
//     handlers: Vec<Box<dyn EventHandler<Event, Error = Error>>>,
// }

// impl<Event, Error> EventHandlers<Event, Error> {
//     pub fn register(&mut self, handler: impl EventHandler<Event, Error = Error> + 'static) {
//         self.handlers.push(Box::new(handler));
//     }

//     pub fn handle(&mut self, event: impl Into<Event>) -> Result<(), Error> {
//         for handler in self.handlers.iter_mut() {
//             handler.handle(&event)?;
//         }
//         Ok(())
//     }
// }
