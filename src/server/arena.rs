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

use crate::server::{ArenaId, ArenaLine, ArenaPoint, Direction, Player, PlayerId};

const ARENA_WIDTH: f64 = 560.0;
const ARENA_HEIGHT: f64 = 560.0;
const ARENA_MAX_PLAYERS: usize = 8;
const ARENA_START_TIMER_SECONDS: i64 = 1;
const LIGHTCYCLE_SPEED: f64 = 55.0;
// const LIGHTCYCLE_BRAKE_SPEED: f64 = 40.0;
// const LIGHTCYCLE_BOOST_SPEED: f64 = 70.0;

#[derive(Debug, Clone, Serialize)]
pub struct Arena {
    pub id: ArenaId,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub max_players: usize,

    pub started: Option<DateTime<Utc>>,
    pub winner: Option<PlayerId>,

    pub players: HashMap<PlayerId, Player>,
    pub lightcycles: HashMap<PlayerId, Lightcycle>,
    pub lightribbons: HashMap<PlayerId, Lightribbon>,

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
        let updates = mem::take(&mut self.updates);
        for update in updates.iter().skip(self.updates_applied_so_far) {
            update.apply(self);
            self.updates_applied_so_far += 1;
        }
        self.updates = updates;

        self
    }

    pub fn process_input(&mut self, player_id: PlayerId, input_event: ArenaInput) {
        input_event
            .process_into_updates(self, player_id)
            .drain(..)
            .for_each(|update| self.updates.push(update));
    }

    pub fn update(&mut self, delta_time: f64) {
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

        self.update_lightcycle_positions(delta_time)
            .apply_updates()
            .calculate_lightcycle_collisions(delta_time)
            .apply_updates()
            .update_lightribbon_positions()
            .apply_updates()
            .test_win_condition()
            .apply_updates()
            .test_round_end()
            .apply_updates();
    }

    //
    // update helpers
    //

    fn update_lightcycle_positions(&mut self, delta_time: f64) -> &mut Self {
        for (id, lightcycle) in self.lightcycles.iter() {
            if lightcycle.dead {
                continue;
            };

            self.updates.push(ArenaUpdate::UpdateLightcyclePosition(
                *id,
                lightcycle.position
                    + lightcycle.direction.as_velocity() * lightcycle.speed * delta_time,
            ))
        }
        self
    }

    fn calculate_lightcycle_collisions(&mut self, delta_time: f64) -> &mut Self {
        'next_lightcycle: for (id, lightcycle) in self.lightcycles.iter() {
            if lightcycle.dead {
                continue 'next_lightcycle;
            };

            let last_position = lightcycle.position
                - lightcycle.direction.as_velocity() * lightcycle.speed * delta_time;

            let travelled = ArenaLine {
                from: last_position.to_untyped(),
                to: lightcycle.position.to_untyped(),
            };

            // test for lightribbon collisions
            for lightribbon in self.lightribbons.values() {
                for line in lightribbon.points.windows(2) {
                    let line = ArenaLine {
                        from: line[0].to_untyped(),
                        to: line[1].to_untyped(),
                    };

                    if travelled.overlaps_segment(&line) {
                        self.updates
                            .push(ArenaUpdate::UpdateLightcycleApplyDeath(*id));
                        continue 'next_lightcycle;
                    }

                    if let Some(intersection) = travelled.intersection(&line) {
                        self.updates.push(ArenaUpdate::UpdateLightcyclePosition(
                            *id,
                            ArenaPoint::from_untyped(intersection),
                        ));
                        self.updates
                            .push(ArenaUpdate::UpdateLightcycleApplyDeath(*id));
                        continue 'next_lightcycle;
                    }
                }
            }

            // test for arena bounds collisions
            if lightcycle.position.x < 0.0
                || lightcycle.position.y < 0.0
                || lightcycle.position.x > self.width
                || lightcycle.position.y > self.height
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

    fn update_lightribbon_positions(&mut self) -> &mut Self {
        for id in self.lightribbons.keys() {
            let latest_point = match self.lightcycles.get(id) {
                Some(lightcycle) => {
                    if lightcycle.dead {
                        continue;
                    };

                    lightcycle.position
                }
                None => {
                    error!(
                        "Failed to update lightribbon position: No lightcycle with id {}",
                        id
                    );
                    continue;
                }
            };

            self.updates
                .push(ArenaUpdate::UpdateLightribbonReplaceLatestPoint(
                    *id,
                    latest_point,
                ));
        }
        self
    }

    fn test_win_condition(&mut self) -> &mut Self {
        if self.winner.is_some() {
            return self;
        }

        if self.lightcycles.iter().count() <= 1 {
            return self;
        }

        let mut alive_lightcycles = self
            .lightcycles
            .iter()
            .filter(|(_, lightcycle)| !lightcycle.dead);

        if let (Some((player_id, _)), None) = (alive_lightcycles.next(), alive_lightcycles.next()) {
            self.updates.push(ArenaUpdate::SetWinner(Some(*player_id)));
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
            id: Default::default(),
            name: Default::default(),
            width: ARENA_WIDTH,
            height: ARENA_HEIGHT,
            max_players: ARENA_MAX_PLAYERS,

            started: None,
            winner: None,

            players: Default::default(),
            lightcycles: Default::default(),
            lightribbons: Default::default(),

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
