use core::fmt::Debug;
use std::{fs, io, io::Write, path::PathBuf};

use itertools::Itertools;
use serde::{de::DeserializeOwned, Serialize};

use crate::prelude::*;

/// A Mapping simply contains functions for mapping
///
/// Invariants:
///
/// commutativity: map_state(apply(x, event)) == transition(map_state(x), map_event(event))
///
/// idempotency: `self` should only be mutated to make the mapping possible,
///               but mappings themselves must be idempotent.
///
pub trait ModelMapping
where
    Self::Model: Machine,
{
    type Model;
    type System;
    type Event;

    fn map_state(&mut self, system: &Self::System) -> Option<StateOf<Self::Model>>;
    fn map_event(&mut self, event: &Self::Event) -> Option<ActionOf<Self::Model>>;
}

pub type StateOf<M> = <M as Machine>::State;
pub type ActionOf<M> = <M as Machine>::Action;
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

    pub fn write_event<'a>(&mut self, event: &'a M::Event) -> io::Result<()>
    where
        M::Event: 'a,
    {
        let action = self.mapping.map_event(event);

        if let Some(action) = action {
            self.write_action(action)?;
        } else {
            tracing::warn!("no action for event: {event:?}");
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
