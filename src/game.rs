mod player;

pub use player::{Player, PlayerJoinable};

use serde_derive::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

const MAX_PLAYERS_PER_GAME: usize = 8;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Game {
    pub id: Uuid,
    pub name: String,
    pub players: HashMap<Uuid, Player>,
    pub max_players: usize,
}

impl Game {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            max_players: MAX_PLAYERS_PER_GAME,
            ..Default::default()
        }
    }
}

impl Game {
    pub fn is_full(&self) -> bool {
        self.max_players <= self.players.len()
    }
}

impl PlayerJoinable for Game {
    fn players(&self) -> &HashMap<Uuid, Player> {
        &self.players
    }

    fn join_player(&mut self, player: Player) {
        self.players.insert(player.id().clone(), player);
    }

    fn part_player(&mut self, uuid: &Uuid) -> Option<Player> {
        self.players.remove(uuid)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct GameState {
    players: Vec<Player>,
}
