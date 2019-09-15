use failure::{format_err, Error};
use log::error;
use nalgebra::{Point2, Vector2};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{GameInputMessage, PlayerColor};

const ARENA_WIDTH: usize = 800;
const ARENA_HEIGHT: usize = 800;
const LIGHTCYCLE_SPEED: isize = 120;

#[derive(Debug, Default, Serialize)]
pub struct Arena {
    width: usize,
    height: usize,

    lightcycles: HashMap<Uuid, Lightcycle>,
    trails: HashMap<Uuid, Trail>,
}

#[derive(Debug, Serialize)]
pub struct Lightcycle {
    position: Point2<isize>,
    rotation: Direction,
    speed: isize,
    color: PlayerColor,
    dead: bool,
}

impl Default for Lightcycle {
    fn default() -> Self {
        Self {
            position: Point2::origin(),
            rotation: Direction::Up,
            speed: LIGHTCYCLE_SPEED,
            color: Default::default(),
            dead: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize)]
pub struct Trail {
    points: Vec<Point2<isize>>,
    color: PlayerColor,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            width: ARENA_WIDTH,
            height: ARENA_HEIGHT,
            ..Default::default()
        }
    }

    pub fn process_input(&mut self, input_events: Vec<(Uuid, GameInputMessage)>) {
        for (client_id, input_event) in input_events {
            match input_event {
                // TODO: Make new GameInput type that is specific to arena input
                // GameInputMessage::StartGame is not relevant here, should be
                // consumed by the game or match system
                GameInputMessage::StartGame => unimplemented!(),
                GameInputMessage::Turn(direction) => {
                    self.turn_lightcycle(client_id, direction)
                        .unwrap_or_else(|error| error!("Failed to turn lightcycle: {}", error));
                }
            }
        }
    }

    pub fn update(&mut self) {
        self.update_lightcycle_positions()
            .update_trail_positions()
            .calculate_lightcycle_collisions();
    }

    //
    // process_input helpers
    //

    fn turn_lightcycle(&mut self, id: Uuid, direction: Direction) -> Result<(), Error> {
        let lightcycle = self
            .lightcycles
            .get_mut(&id)
            .ok_or_else(|| format_err!("No lightcycle with id {}", id))?;

        // TODO: Prevent turning 180° in one update (will immediately crash into own trail)
        lightcycle.rotation = direction;

        Ok(())
    }

    //
    // update helpers
    //

    fn update_lightcycle_positions(&mut self) -> &mut Self {
        for lightcycle in self.lightcycles.values_mut() {
            if lightcycle.dead {
                continue;
            };

            lightcycle.position += lightcycle.rotation.as_velocity() * lightcycle.speed;
        }
        self
    }

    fn update_trail_positions(&mut self) -> &mut Self {
        for (id, trail) in self.trails.iter_mut() {
            let latest_point = match self.lightcycles.get(id) {
                Some(lightcycle) => {
                    if lightcycle.dead {
                        continue;
                    };

                    lightcycle.position
                }
                None => {
                    error!(
                        "Failed to update trail position: No lightcycle with id {}",
                        id
                    );
                    continue;
                }
            };

            trail.points.pop();
            trail.points.push(latest_point.clone());
        }
        self
    }

    fn calculate_lightcycle_collisions(&mut self) -> &mut Self {
        for lightcycle in self.lightcycles.values_mut() {
            if lightcycle.dead {
                continue;
            };

            for trail in self.trails.values() {
                for line in trail.points.windows(2) {
                    let start = line[0];
                    let end = line[1];

                    if is_point_on_line_2d(lightcycle.position, (start, end)) {
                        lightcycle.dead = true;
                        continue;
                    }
                }
            }
        }

        self
    }
}

fn is_point_on_line_2d(point: Point2<isize>, line: (Point2<isize>, Point2<isize>)) -> bool {
    let (start, end) = line;

    if start.x == end.x {
        if point.x != start.x {
            return false;
        }

        return is_point_on_line_1d(point.y, (start.y, end.y));
    }

    if start.y == end.y {
        if point.y != start.y {
            return false;
        }

        return is_point_on_line_1d(point.x, (start.x, end.x));
    }

    false
}

fn is_point_on_line_1d(point: isize, line: (isize, isize)) -> bool {
    let (low, high) = if line.0 <= line.1 {
        (line.0, line.1)
    } else {
        (line.1, line.0)
    };

    if point < low || high < point {
        return false;
    }

    true
}
