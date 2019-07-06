use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

pub trait PlayerJoinable {
    fn players(&self) -> &HashSet<Uuid>;
    fn add_player(&mut self, uuid: &Uuid);
    fn remove_player(&mut self, uuid: &Uuid) -> bool;
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
