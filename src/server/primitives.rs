use actix::Recipient;
use euclid::{Point2D, Vector2D};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::MessageOut;

pub type ClientId = Uuid;
pub type PlayerId = Uuid;
pub type ArenaId = Uuid;

/// The euclidian space in which ArenaVectors and ArenaPoints operate.
pub struct ArenaSpace;
/// Represents a direction in the ArenaSpace
pub type ArenaVector = Vector2D<isize, ArenaSpace>;
/// Represents a position in the ArenaSpace
pub type ArenaPoint = Point2D<isize, ArenaSpace>;
/// Represents a line between two points in the ArenaSpace
pub struct Line(pub ArenaPoint, pub ArenaPoint);

#[derive(Debug)]
pub struct Client {
    pub id: ClientId,
    pub ip_address: Option<String>,
    pub address: Recipient<MessageOut>,
    pub player: Option<PlayerId>,
    pub arena: Option<ArenaId>,
    pub updates_sent_so_far: usize,
}

#[derive(Debug, Clone, Hash, PartialEq, Serialize, Deserialize)]
pub struct Player {
    #[serde(skip_deserializing)]
    pub id: PlayerId,
    pub name: String,
    pub color: PlayerColor,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: Default::default(),
            color: Default::default(),
        }
    }
}

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
    pub fn as_velocity(self) -> ArenaVector {
        match self {
            Direction::Up => ArenaVector::new(0, 1),
            Direction::Down => ArenaVector::new(0, -1),
            Direction::Left => ArenaVector::new(-1, 0),
            Direction::Right => ArenaVector::new(1, 0),
        }
    }

    pub fn is_opposite(self, to: Direction) -> bool {
        match self {
            Direction::Up => to == Direction::Down,
            Direction::Left => to == Direction::Right,
            Direction::Right => to == Direction::Left,
            Direction::Down => to == Direction::Up,
        }
    }
}
