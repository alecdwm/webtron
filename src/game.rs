mod lobby;
mod player;

pub use lobby::Lobby;
pub use player::{Player, PlayerJoinable};

use serde_derive::Serialize;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Game {
    pub id: Uuid,
    pub name: String,
    pub players: HashSet<Uuid>,
    pub max_players: usize,
}

impl Game {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_owned(),
            max_players: 8,
            ..Default::default()
        }
    }
}

impl PlayerJoinable for Game {
    fn players(&self) -> &HashSet<Uuid> {
        &self.players
    }

    fn add_player(&mut self, uuid: &Uuid) {
        self.players.insert(uuid.clone());
    }

    fn remove_player(&mut self, uuid: &Uuid) -> bool {
        self.players.remove(uuid)
    }
}
