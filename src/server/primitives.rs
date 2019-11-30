use crate::server::Player;
use euclid::{Point2D, Vector2D};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

pub type ClientId = Uuid;
pub type PlayerId = Uuid;
pub type GameId = Uuid;

pub struct ArenaSpace;
pub type ArenaVector = Vector2D<isize, ArenaSpace>;
pub type ArenaPoint = Point2D<isize, ArenaSpace>;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Copy, Clone, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Left,
    Right,
    Down,
}

impl Direction {
    pub fn as_velocity(&self) -> ArenaVector {
        match self {
            Direction::Up => ArenaVector::new(0, 1),
            Direction::Down => ArenaVector::new(0, -1),
            Direction::Left => ArenaVector::new(-1, 0),
            Direction::Right => ArenaVector::new(1, 0),
        }
    }
}

#[derive(Debug, Default, Clone, Hash, PartialEq, Serialize, Deserialize)]
pub struct NetworkPlayer {
    #[serde(skip_deserializing)]
    pub id: PlayerId,
    pub name: String,
    pub color: PlayerColor,
}

impl From<&Player> for NetworkPlayer {
    fn from(player: &Player) -> Self {
        Self {
            id: player.id,
            name: player.name.clone(),
            color: player.color,
        }
    }
}
