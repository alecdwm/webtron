use super::*;

#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub struct Lightcycle {
    pub position: ArenaPoint,
    pub direction: Direction,
    pub speed: f64,
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

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Lightribbon {
    pub points: Vec<ArenaPoint>,
}
