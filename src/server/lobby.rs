use super::game::Game;
use debug_stub_derive::DebugStub;
use failure::{format_err, Error};
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::thread;
use std::time;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct Lobby {
    games: HashMap<Uuid, Game>,
    players: HashSet<Uuid>,
}

impl Lobby {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn games(&self) -> &HashMap<Uuid, Game> {
        &self.games
    }

    pub fn players(&self) -> &HashSet<Uuid> {
        &self.players
    }

    pub fn players_mut(&mut self) -> &mut HashSet<Uuid> {
        &mut self.players
    }

    pub fn new_game(&mut self, name: &str) {
        let game = Game::new(name);
        self.games.insert(game.id, game);
    }

    pub fn move_player_to_game(&mut self, game_uuid: Uuid, player_uuid: Uuid) -> Result<(), Error> {
        for game in self.games.values_mut() {
            if game.id == game_uuid {
                continue;
            };
            game.remove_player(&player_uuid);
        }
        self.remove_player(&player_uuid);

        self.games
            .get_mut(&game_uuid)
            .ok_or_else(|| {
                format_err!(
                    "Failed to move player to game with uuid '{}': not found",
                    game_uuid
                )
            })?
            .add_player(player_uuid);

        Ok(())
    }

    pub fn move_player_to_lobby(&mut self, uuid: Uuid) {
        for game in self.games.values_mut() {
            game.remove_player(&uuid);
        }
        self.add_player(uuid);
    }

    pub fn add_player(&mut self, uuid: Uuid) {
        self.players.insert(uuid);
    }

    pub fn remove_player(&mut self, uuid: &Uuid) {
        self.players.remove(uuid);
    }
}
