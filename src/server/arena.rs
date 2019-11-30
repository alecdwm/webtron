use log::error;
use rand_core::{OsRng, RngCore};
use serde_derive::Serialize;
use std::collections::HashMap;

use crate::server::{ArenaPoint, Direction, PlayerId, MAX_PLAYERS_PER_GAME};

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
    position: ArenaPoint,
    rotation: Direction,
    speed: isize,
    dead: bool,
}

impl Default for Lightcycle {
    fn default() -> Self {
        Self {
            position: ArenaPoint::origin(),
            rotation: Direction::Up,
            speed: LIGHTCYCLE_SPEED,
            dead: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Hash, PartialEq, Serialize)]
pub struct Trail {
    points: Vec<ArenaPoint>,
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
    updates_applied_so_far: usize,
}

impl ArenaUpdates {
    pub fn push(&mut self, update: ArenaUpdate) {
        self.updates.push(update);
    }

    pub fn clear(&mut self) {
        self.updates.clear();
        self.updates_applied_so_far = 0;
    }

    pub fn apply<'s, 'arena>(&'s mut self, arena: &'arena mut Arena) -> &'arena mut Arena {
        for update in self.updates.iter().skip(self.updates_applied_so_far) {
            update.apply(arena);
            self.updates_applied_so_far += 1;
        }

        arena
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Serialize)]
pub enum ArenaUpdate {
    AddLightcycle(PlayerId, Lightcycle),
    AddTrail(PlayerId, Trail),

    // UpdateLightcycle(PlayerId, Lightcycle),
    UpdateLightcycleChangeDirection(PlayerId, Direction),
    UpdateLightcycleApplyVelocity(PlayerId),
    UpdateLightcycleApplyDeath(PlayerId),

    // UpdateTrail(PlayerId, Trail),
    UpdateTrailAppendPoint(PlayerId, ArenaPoint),
    UpdateTrailReplaceLatestPoint(PlayerId, ArenaPoint),

    RemoveLightcycle(PlayerId),
    RemoveTrail(PlayerId),
}

impl ArenaUpdate {
    pub fn apply<'s, 'arena>(&'s self, arena: &'arena mut Arena) -> &'arena mut Arena {
        match self {
            ArenaUpdate::AddLightcycle(player_id, lightcycle) => {
                arena.lightcycles.insert(*player_id, *lightcycle);
            }

            ArenaUpdate::AddTrail(player_id, trail) => {
                arena.trails.insert(*player_id, trail.clone());
            }

            ArenaUpdate::UpdateLightcycleChangeDirection(player_id, direction) => {
                let lightcycle = match arena.lightcycles.get_mut(player_id) {
                    Some(lightcycle) => lightcycle,
                    None => {
                        error!("Lightcycle {} not found", player_id);
                        return arena;
                    }
                };

                lightcycle.rotation = *direction;
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
    pub fn process_input(
        &mut self,
        updates: &mut ArenaUpdates,
        input_event: (PlayerId, ArenaInput),
    ) {
        let (player_id, input_event) = input_event;
        match input_event {
            ArenaInput::SpawnPlayers(player_ids) => {
                self.process_input_spawn_players(updates, player_ids);
            }

            ArenaInput::Turn(direction) => {
                self.process_input_turn_lightcycle(updates, player_id, direction);
            }
        }
    }

    pub fn update(&mut self, updates: &mut ArenaUpdates) {
        // apply process_input updates
        updates.apply(self);

        self.update_lightcycle_positions(updates)
            .apply(self)
            .update_trail_positions(updates)
            .apply(self)
            .calculate_lightcycle_collisions(updates)
            .apply(self);
    }

    //
    // process_input helpers
    //

    fn process_input_spawn_players(
        &mut self,
        updates: &mut ArenaUpdates,
        player_ids: Vec<PlayerId>,
    ) {
        calculate_spawnpoints(player_ids).drain(..).for_each(
            |(player_id, spawn_position, spawn_direction)| {
                updates.push(ArenaUpdate::AddLightcycle(
                    player_id,
                    Lightcycle {
                        position: spawn_position,
                        rotation: spawn_direction,
                        ..Default::default()
                    },
                ));
                updates.push(ArenaUpdate::AddTrail(
                    player_id,
                    Trail {
                        points: vec![spawn_position, spawn_position],
                    },
                ));
            },
        );
    }

    fn process_input_turn_lightcycle(
        &mut self,
        updates: &mut ArenaUpdates,
        player_id: PlayerId,
        direction: Direction,
    ) {
        // TODO: Prevent turning 180° in one update (will immediately crash into own trail)
        updates.push(ArenaUpdate::UpdateLightcycleChangeDirection(
            player_id, direction,
        ));
    }

    //
    // update helpers
    //

    fn update_lightcycle_positions<'s, 'updates>(
        &'s self,
        updates: &'updates mut ArenaUpdates,
    ) -> &'updates mut ArenaUpdates {
        for (id, lightcycle) in self.lightcycles.iter() {
            if lightcycle.dead {
                continue;
            };

            updates.push(ArenaUpdate::UpdateLightcycleApplyVelocity(id.clone()))
        }
        updates
    }

    fn update_trail_positions<'s, 'updates>(
        &'s self,
        updates: &'updates mut ArenaUpdates,
    ) -> &'updates mut ArenaUpdates {
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

    fn calculate_lightcycle_collisions<'s, 'updates>(
        &'s self,
        updates: &'updates mut ArenaUpdates,
    ) -> &'updates mut ArenaUpdates {
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

const SPAWNPOINTS: [(isize, isize, Direction); MAX_PLAYERS_PER_GAME] = [
    // (
    //     ARENA_WIDTH as isize / 2,
    //     ARENA_HEIGHT as isize / 2,
    //     Direction::Up,
    // ),
    //
    (
        ARENA_WIDTH as isize / 2,
        ARENA_HEIGHT as isize / 2 - ARENA_HEIGHT as isize / 4,
        Direction::Down,
    ),
    (
        ARENA_WIDTH as isize / 2,
        ARENA_HEIGHT as isize / 2 + ARENA_HEIGHT as isize / 4,
        Direction::Up,
    ),
    //
    (
        ARENA_WIDTH as isize / 2 - ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2,
        Direction::Right,
    ),
    (
        ARENA_WIDTH as isize / 2 + ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2,
        Direction::Left,
    ),
    //
    (
        ARENA_WIDTH as isize / 2 - ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2 - ARENA_HEIGHT as isize / 4,
        Direction::Right,
    ),
    (
        ARENA_WIDTH as isize / 2 - ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2 + ARENA_HEIGHT as isize / 4,
        Direction::Right,
    ),
    (
        ARENA_WIDTH as isize / 2 + ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2 - ARENA_HEIGHT as isize / 4,
        Direction::Left,
    ),
    (
        ARENA_WIDTH as isize / 2 + ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2 + ARENA_HEIGHT as isize / 4,
        Direction::Left,
    ),
];

fn calculate_spawnpoints(player_ids: Vec<PlayerId>) -> Vec<(PlayerId, ArenaPoint, Direction)> {
    let mut spawnpoints: Vec<(PlayerId, ArenaPoint, Direction)> = Vec::new();
    let mut spawnpoints_used: Vec<usize> = Vec::new();

    for player_id in player_ids {
        // check if no spawnpoints remain
        if spawnpoints_used.len() >= SPAWNPOINTS.len() {
            error!("No spawnpoints remain!");
            break;
        }

        // randomly select an available spawnpoint
        let mut selected_spawnpoint = OsRng.next_u64() as usize % SPAWNPOINTS.len();
        while spawnpoints_used
            .iter()
            .any(|spawnpoint| *spawnpoint == selected_spawnpoint)
        {
            selected_spawnpoint = OsRng.next_u64() as usize % SPAWNPOINTS.len();
        }
        spawnpoints_used.push(selected_spawnpoint);

        // add the selected spawnpoint to our spawnpoints vector
        let selected_spawnpoint = SPAWNPOINTS[selected_spawnpoint];
        spawnpoints.push((
            player_id,
            ArenaPoint::new(selected_spawnpoint.0, selected_spawnpoint.1),
            selected_spawnpoint.2,
        ));
    }

    spawnpoints
}

struct Line(ArenaPoint, ArenaPoint);

fn is_point_on_line_2d(point: ArenaPoint, line: Line) -> bool {
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
