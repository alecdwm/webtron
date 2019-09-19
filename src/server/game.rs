use serde_derive::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

use super::{Player, PlayerJoinable};

const MAX_PLAYERS_PER_GAME: usize = 8;

#[derive(Debug, Clone, Serialize)]
pub struct Game {
    pub id: Uuid,
    players: HashMap<Uuid, Player>,
    max_players: usize,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            players: Default::default(),
            max_players: MAX_PLAYERS_PER_GAME,
        }
    }
}

impl Game {
    pub fn is_full(&self) -> bool {
        self.max_players <= self.players.len()
    }

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }
}

impl PlayerJoinable for Game {
    fn players(&self) -> &HashMap<Uuid, Player> {
        &self.players
    }

    fn join_player(&mut self, player: Player) {
        self.players.insert(player.id.clone(), player);
    }

    fn part_player(&mut self, uuid: &Uuid) -> Option<Player> {
        self.players.remove(uuid)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct GameState {
    players: Vec<Player>,
}
