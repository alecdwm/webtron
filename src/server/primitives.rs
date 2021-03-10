use euclid::{Point2D, Vector2D};
use lyon_geom::LineSegment;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::new_id_type;
use crate::server::MessageOut;

new_id_type!(ClientId);
new_id_type!(PlayerId);
new_id_type!(ArenaId);

/// The euclidian space in which ArenaVectors and ArenaPoints operate.
pub struct ArenaSpace;
/// Represents a direction in the ArenaSpace
pub type ArenaVector = Vector2D<f64, ArenaSpace>;
/// Represents a position in the ArenaSpace
pub type ArenaPoint = Point2D<f64, ArenaSpace>;
/// Represents a line between two points in the ArenaSpace
pub type ArenaLine = LineSegment<f64>;

#[derive(Debug)]
pub struct Client {
    pub id: ClientId,
    pub ip_address: Option<String>,
    pub tx: Sender<MessageOut>,
    pub player: Option<PlayerId>,
    pub arena: Option<ArenaId>,
    pub updates_sent_so_far: usize,
}

#[derive(Default, Debug, Clone, Hash, PartialEq, Serialize, Deserialize)]
pub struct Player {
    #[serde(skip_serializing, skip_deserializing)]
    pub id: PlayerId,
    pub name: String,
    pub color: PlayerColor,
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
            Direction::Up => ArenaVector::new(0.0, 1.0),
            Direction::Down => ArenaVector::new(0.0, -1.0),
            Direction::Left => ArenaVector::new(-1.0, 0.0),
            Direction::Right => ArenaVector::new(1.0, 0.0),
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
