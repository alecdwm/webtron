use super::*;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Serialize)]
pub struct Lightcycle {
    pub position: ArenaPoint,
    pub direction: Direction,
    pub speed: isize,
    pub dead: bool,
}

impl Default for Lightcycle {
    fn default() -> Self {
        Self {
            position: ArenaPoint::origin(),
            direction: Direction::Up,
            speed: LIGHTCYCLE_SPEED,
            dead: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Hash, PartialEq, Serialize)]
pub struct Lightribbon {
    pub points: Vec<ArenaPoint>,
}
