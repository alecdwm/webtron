use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

pub trait PlayerJoinable {
    fn players(&self) -> &HashSet<Uuid>;
    fn add_player(&mut self, uuid: &Uuid);
    fn remove_player(&mut self, uuid: &Uuid);
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Player {
    name: String,
    color: PlayerColor,
}

#[derive(Debug, Serialize, Deserialize)]
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
