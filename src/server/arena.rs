use failure::{format_err, Error};
use log::error;
use nalgebra::Point2;
use serde_derive::Serialize;
use std::collections::HashMap;

use crate::get_error_chain;
use crate::server::{Direction, PlayerId};

const ARENA_WIDTH: usize = 800;
const ARENA_HEIGHT: usize = 800;
const LIGHTCYCLE_SPEED: isize = 120;

//
// arena datastructure
//

#[derive(Debug, Clone, Serialize)]
pub struct Arena {
    width: usize,
    height: usize,

    lightcycles: HashMap<PlayerId, Lightcycle>,
    trails: HashMap<PlayerId, Trail>,
}

impl Default for Arena {
    fn default() -> Self {
        Self {
            width: ARENA_WIDTH,
            height: ARENA_HEIGHT,
            lightcycles: Default::default(),
            trails: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Serialize)]
pub struct Lightcycle {
    position: Point2<isize>,
    rotation: Direction,
    speed: isize,
    dead: bool,
}

impl Default for Lightcycle {
    fn default() -> Self {
        Self {
            position: Point2::origin(),
            rotation: Direction::Up,
            speed: LIGHTCYCLE_SPEED,
            dead: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Hash, PartialEq, Serialize)]
pub struct Trail {
    points: Vec<Point2<isize>>,
}

//
// arena input + update events
//

pub enum ArenaInput {
    SpawnPlayers(Vec<PlayerId>),
    Turn(Direction),
}

#[derive(Debug, Default, Clone, Hash, PartialEq, Serialize)]
pub struct ArenaUpdates {
    updates: Vec<ArenaUpdate>,
}

impl ArenaUpdates {
    pub fn push(&mut self, update: ArenaUpdate) {
        self.updates.push(update);
    }

    pub fn apply<'a, 'b>(&'a self, arena: &'b mut Arena) -> &'b mut Arena {
        for update in self.updates.iter() {
            update.apply(arena);
        }

        arena
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Serialize)]
pub enum ArenaUpdate {
    AddLightcycle(PlayerId, Lightcycle),
    AddTrail(PlayerId, Trail),

    // UpdateLightcycle(PlayerId, Lightcycle),
    UpdateLightcycleApplyVelocity(PlayerId),
    UpdateLightcycleApplyDeath(PlayerId),

    // UpdateTrail(PlayerId, Trail),
    UpdateTrailAppendPoint(PlayerId, Point2<isize>),
    UpdateTrailReplaceLatestPoint(PlayerId, Point2<isize>),

    RemoveLightcycle(PlayerId),
    RemoveTrail(PlayerId),
}

impl ArenaUpdate {
    pub fn apply<'a, 'b>(&'a self, arena: &'b mut Arena) -> &'b mut Arena {
        match self {
            ArenaUpdate::AddLightcycle(player_id, lightcycle) => {
                arena.lightcycles.insert(*player_id, *lightcycle);
            }

            ArenaUpdate::AddTrail(player_id, trail) => {
                arena.trails.insert(*player_id, trail.clone());
            }

            ArenaUpdate::UpdateLightcycleApplyVelocity(player_id) => {
                let lightcycle = match arena.lightcycles.get_mut(player_id) {
                    Some(lightcycle) => lightcycle,
                    None => {
                        error!("Lightcycle {} not found", player_id);
                        return arena;
                    }
                };

                lightcycle.position += lightcycle.rotation.as_velocity() * lightcycle.speed;
            }

            ArenaUpdate::UpdateLightcycleApplyDeath(player_id) => {
                let lightcycle = match arena.lightcycles.get_mut(player_id) {
                    Some(lightcycle) => lightcycle,
                    None => {
                        error!("Lightcycle {} not found", player_id);
                        return arena;
                    }
                };

                lightcycle.dead = true;
            }

            ArenaUpdate::UpdateTrailAppendPoint(player_id, point) => {
                let trail = match arena.trails.get_mut(player_id) {
                    Some(trail) => trail,
                    None => {
                        error!("Trail {} not found", player_id);
                        return arena;
                    }
                };

                trail.points.push(*point);
            }

            ArenaUpdate::UpdateTrailReplaceLatestPoint(player_id, latest_point) => {
                let trail = match arena.trails.get_mut(player_id) {
                    Some(trail) => trail,
                    None => {
                        error!("Trail {} not found", player_id);
                        return arena;
                    }
                };

                trail.points.pop();
                trail.points.push(*latest_point);
            }

            ArenaUpdate::RemoveLightcycle(player_id) => {
                arena.lightcycles.remove(player_id);
            }

            ArenaUpdate::RemoveTrail(player_id) => {
                arena.trails.remove(player_id);
            }
        }

        arena
    }
}

impl Arena {
    pub fn process_input(&mut self, input_events: Vec<(PlayerId, ArenaInput)>) {
        for (player_id, input_event) in input_events {
            match input_event {
                ArenaInput::SpawnPlayers(player_ids) => {
                    for player_id in player_ids {
                        self.spawn_player(player_id).unwrap_or_else(|error| {
                            error!("Failed to spawn player: {}", get_error_chain(error))
                        });
                    }
                }

                ArenaInput::Turn(direction) => {
                    self.turn_lightcycle(player_id, direction)
                        .unwrap_or_else(|error| {
                            error!("Failed to turn lightcycle: {}", get_error_chain(error))
                        });
                }
            }
        }
    }

    pub fn update(&mut self) -> ArenaUpdates {
        let mut updates = ArenaUpdates::default();

        self.update_lightcycle_positions(&mut updates)
            .apply(self)
            .update_trail_positions(&mut updates)
            .apply(self)
            .calculate_lightcycle_collisions(&mut updates)
            .apply(self);

        updates
    }

    //
    // process_input helpers
    //

    fn spawn_player(&mut self, id: PlayerId) -> Result<(), Error> {
        unimplemented!();
    }

    fn turn_lightcycle(&mut self, id: PlayerId, direction: Direction) -> Result<(), Error> {
        let lightcycle = self
            .lightcycles
            .get_mut(&id)
            .ok_or_else(|| format_err!("No lightcycle with id {}", id))?;

        // TODO: Create an ArenaUpdate variant for this so that it can be synced over the network
        // TODO: Prevent turning 180° in one update (will immediately crash into own trail)
        lightcycle.rotation = direction;

        Ok(())
    }

    //
    // update helpers
    //

    fn update_lightcycle_positions<'a, 'b>(
        &'a self,
        updates: &'b mut ArenaUpdates,
    ) -> &'b mut ArenaUpdates {
        for (id, lightcycle) in self.lightcycles.iter() {
            if lightcycle.dead {
                continue;
            };

            updates.push(ArenaUpdate::UpdateLightcycleApplyVelocity(id.clone()))
        }
        updates
    }

    fn update_trail_positions<'a, 'b>(
        &'a self,
        updates: &'b mut ArenaUpdates,
    ) -> &'b mut ArenaUpdates {
        for id in self.trails.keys() {
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

            updates.push(ArenaUpdate::UpdateTrailReplaceLatestPoint(
                id.clone(),
                latest_point,
            ));
        }
        updates
    }

    fn calculate_lightcycle_collisions<'a, 'b>(
        &'a self,
        updates: &'b mut ArenaUpdates,
    ) -> &'b mut ArenaUpdates {
        'next_lightcycle: for (id, lightcycle) in self.lightcycles.iter() {
            if lightcycle.dead {
                continue 'next_lightcycle;
            };

            for trail in self.trails.values() {
                for line in trail.points.windows(2) {
                    let start = line[0];
                    let end = line[1];

                    if is_point_on_line_2d(lightcycle.position, Line(start, end)) {
                        updates.push(ArenaUpdate::UpdateLightcycleApplyDeath(id.clone()));
                        continue 'next_lightcycle;
                    }
                }
            }
        }
        updates
    }
}

struct Line(Point2<isize>, Point2<isize>);

fn is_point_on_line_2d(point: Point2<isize>, line: Line) -> bool {
    let start = line.0;
    let end = line.1;

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
