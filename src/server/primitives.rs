use euclid::{Point2D, Vector2D};
use lyon_geom::LineSegment;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::time::Instant;

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
    pub tx: Sender<MessageOut>,
    pub player: Option<PlayerId>,
    pub arena: Option<ArenaId>,
    /// Whether the client has been sent its arena's full state, after which
    /// it is only sent each update's changes.
    pub synced: bool,
    /// The player name this client has claimed, normalised.
    pub name: Option<String>,
    pub connected_at: Instant,
    /// What the player has done since connecting, for the ntfy message sent
    /// when they disconnect.
    pub session: SessionStats,
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct SessionStats {
    pub rounds: u32,
    pub wins: u32,
    pub kills: u32,
}

#[derive(Default, Debug, Clone, Hash, PartialEq, Serialize, Deserialize)]
pub struct Player {
    #[serde(skip_serializing, skip_deserializing)]
    pub id: PlayerId,
    pub name: String,
    pub color: PlayerColor,
    /// Set by the server for computer-controlled players; clients can't set it.
    #[serde(skip_deserializing)]
    pub npc: bool,
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerColor {
    Blue,
    Green,
    Orange,
    Purple,
    Red,
    #[default]
    White,
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

    pub fn perpendicular_directions(self) -> [Direction; 2] {
        match self {
            Direction::Up | Direction::Down => [Direction::Left, Direction::Right],
            Direction::Left | Direction::Right => [Direction::Up, Direction::Down],
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
