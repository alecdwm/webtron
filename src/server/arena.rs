mod entities;
mod input;
mod updates;
mod util;

use chrono::{DateTime, Duration as OldDuration, Utc};
use log::{error, trace};
use rand_core::{OsRng, RngCore};
use serde_derive::Serialize;
use std::collections::HashMap;
use std::mem;

pub use self::entities::*;
pub use self::input::*;
pub use self::updates::*;
pub use self::util::*;

use crate::server::{ArenaId, ArenaPoint, Direction, Line, Player, PlayerId};

const ARENA_WIDTH: usize = 560;
const ARENA_HEIGHT: usize = 560;
const ARENA_MAX_PLAYERS: usize = 8;
const ARENA_START_TIMER_SECONDS: i64 = 5;
const LIGHTCYCLE_SPEED: isize = 1;

#[derive(Debug, Clone, Serialize)]
pub struct Arena {
    pub id: ArenaId,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub max_players: usize,

    pub started: Option<DateTime<Utc>>,

    pub players: HashMap<PlayerId, Player>,
    pub lightcycles: HashMap<PlayerId, Lightcycle>,
    pub trails: HashMap<PlayerId, Trail>,

    #[serde(skip)]
    pub updates: Vec<ArenaUpdate>,
    #[serde(skip)]
    updates_applied_so_far: usize,
}

impl Arena {
    pub fn with_name(name: &str) -> Self {
        Self {
            name: name.to_uppercase(),
            ..Default::default()
        }
    }

    pub fn add_player(&mut self, player: Player) {
        self.updates.push(ArenaUpdate::AddPlayer(player.id, player));
    }

    pub fn remove_player(&mut self, player_id: PlayerId) {
        self.updates.push(ArenaUpdate::RemovePlayer(player_id));
    }

    pub fn clear_updates(&mut self) {
        self.updates.clear();
        self.updates_applied_so_far = 0;
    }

    pub fn apply_updates(&mut self) -> &mut Self {
        let updates = mem::replace(&mut self.updates, Vec::new());
        for update in updates.iter().skip(self.updates_applied_so_far) {
            update.apply(self);
            self.updates_applied_so_far += 1;
        }
        mem::replace(&mut self.updates, updates);

        self
    }

    pub fn process_input(&mut self, player_id: PlayerId, input_event: ArenaInput) {
        input_event
            .process_into_updates(self, player_id)
            .drain(..)
            .for_each(|update| self.updates.push(update));
    }

    pub fn update(&mut self) {
        // apply process_input updates
        self.apply_updates();

        let started = match self.started {
            Some(started) => started,
            None => return,
        };

        let now = Utc::now();
        if now < started {
            return;
        }

        self.update_lightcycle_positions()
            .apply_updates()
            .calculate_lightcycle_collisions()
            .apply_updates()
            .update_trail_positions()
            .apply_updates()
            .test_round_end()
            .apply_updates();
    }

    //
    // update helpers
    //

    fn update_lightcycle_positions(&mut self) -> &mut Self {
        for (id, lightcycle) in self.lightcycles.iter() {
            if lightcycle.dead {
                continue;
            };

            self.updates.push(ArenaUpdate::UpdateLightcyclePosition(
                *id,
                lightcycle.position + lightcycle.direction.as_velocity() * lightcycle.speed,
            ))
        }
        self
    }

    fn calculate_lightcycle_collisions(&mut self) -> &mut Self {
        'next_lightcycle: for (id, lightcycle) in self.lightcycles.iter() {
            if lightcycle.dead {
                continue 'next_lightcycle;
            };

            // test for trail collisions
            for trail in self.trails.values() {
                for line in trail.points.windows(2) {
                    let start = line[0];
                    let end = line[1];

                    if is_point_on_line_2d(lightcycle.position, Line(start, end)) {
                        self.updates
                            .push(ArenaUpdate::UpdateLightcycleApplyDeath(*id));
                        continue 'next_lightcycle;
                    }
                }
            }

            // test for arena bounds collisions
            if lightcycle.position.x < 0
                || lightcycle.position.y < 0
                || lightcycle.position.x > self.width as isize
                || lightcycle.position.y > self.height as isize
            {
                self.updates
                    .push(ArenaUpdate::UpdateLightcycleApplyDeath(*id));
                continue 'next_lightcycle;
            }

            // test for collisions with other lightcycles
            if self
                .lightcycles
                .iter()
                .filter(|(_, other_lightcycle)| !other_lightcycle.dead)
                .filter(|(other_id, _)| id != *other_id)
                .any(|(_, other_lightcycle)| lightcycle.position == other_lightcycle.position)
            {
                self.updates
                    .push(ArenaUpdate::UpdateLightcycleApplyDeath(*id));
                continue 'next_lightcycle;
            }
        }
        self
    }

    fn update_trail_positions(&mut self) -> &mut Self {
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

            self.updates
                .push(ArenaUpdate::UpdateTrailReplaceLatestPoint(
                    *id,
                    latest_point,
                ));
        }
        self
    }

    fn test_round_end(&mut self) -> &mut Self {
        if self.lightcycles.values().all(|lightcycle| lightcycle.dead) {
            self.updates.push(ArenaUpdate::End);
        }
        self
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self {
            id: ArenaId::new_v4(),
            name: Default::default(),
            width: ARENA_WIDTH,
            height: ARENA_HEIGHT,
            max_players: ARENA_MAX_PLAYERS,

            started: None,

            players: Default::default(),
            lightcycles: Default::default(),
            trails: Default::default(),

            updates: Default::default(),
            updates_applied_so_far: 0,
        }
    }
}

/// ArenaOverview represents an overview of an Arena for the arena selection screen.
#[derive(Debug, Clone, Serialize)]
pub struct ArenaOverview {
    id: ArenaId,
    name: String,
    max_players: usize,
    started: Option<DateTime<Utc>>,
    players: HashMap<PlayerId, Player>,
}

impl From<&Arena> for ArenaOverview {
    fn from(arena: &Arena) -> Self {
        Self {
            id: arena.id,
            name: arena.name.clone(),
            max_players: arena.max_players,
            started: arena.started,
            players: arena.players.clone(),
        }
    }
}
