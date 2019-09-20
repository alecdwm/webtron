use nalgebra::Vector2;
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

pub type GameId = Uuid;
pub type ClientId = Uuid;
pub type PlayerId = Uuid;

#[derive(Debug, Clone, Hash, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Left,
    Right,
    Down,
}

impl Direction {
    pub fn as_velocity(&self) -> Vector2<isize> {
        match self {
            Direction::Up => Vector2::new(0, 1),
            Direction::Down => Vector2::new(0, -1),
            Direction::Left => Vector2::new(-1, 0),
            Direction::Right => Vector2::new(1, 0),
        }
    }
}

// #[derive(Debug, Default, Clone, Hash, PartialEq, Serialize, Deserialize)]
// pub struct MessagePlayer {
//     #[serde(skip_deserializing)]
//     pub id: PlayerId,
//     pub name: String,
//     pub color: PlayerColor,
// }

// impl From<super::Player> for NetworkPlayer {
//     fn from(player: super::Player) -> Self {
//         Self {
//             id: player.id,
//             name: player.name,
//             color: player.color,
//         }
//     }
// }
