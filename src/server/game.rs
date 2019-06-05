use debug_stub_derive::DebugStub;
use failure::{format_err, Error};
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::thread;
use std::time;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Game {
    pub id: Uuid,
    pub name: String,
    pub players: HashSet<Uuid>,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            players: HashSet::new(),
        }
    }
}

impl Game {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..Default::default()
        }
    }

    pub fn add_player(&mut self, uuid: Uuid) {
        self.players.insert(uuid);
    }

    pub fn remove_player(&mut self, uuid: &Uuid) {
        self.players.remove(uuid);
    }
}
