use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub trait PlayerJoinable {
    fn players(&self) -> &HashMap<Uuid, Player>;
    fn join_player(&mut self, player: Player);
    fn part_player(&mut self, uuid: &Uuid) -> Option<Player>;
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Player {
    #[serde(skip_deserializing)]
    id: Uuid,
    name: String,
    color: PlayerColor,
}

impl Player {
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn set_id(&mut self, id: Uuid) {
        self.id = id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn color(&self) -> &PlayerColor {
        &self.color
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerColor {
    Blue,
    Green,
    Orange,
    Purple,
    Red,
    White,
}

impl Default for PlayerColor {
    fn default() -> Self {
        PlayerColor::White
    }
}
