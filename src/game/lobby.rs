use super::PlayerJoinable;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct Lobby {
    players: HashSet<Uuid>,
}

impl PlayerJoinable for Lobby {
    fn players(&self) -> &HashSet<Uuid> {
        &self.players
    }

    fn add_player(&mut self, uuid: &Uuid) {
        self.players.insert(uuid.clone());
    }

    fn remove_player(&mut self, uuid: &Uuid) {
        self.players.remove(uuid);
    }
}
