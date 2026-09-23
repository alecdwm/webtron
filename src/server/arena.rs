mod entities;
mod input;
pub mod npc;
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

use crate::server::{ArenaId, ArenaLine, ArenaPoint, Direction, Player, PlayerColor, PlayerId};

const ARENA_WIDTH: f64 = 560.0;
const ARENA_HEIGHT: f64 = 560.0;
const ARENA_MAX_PLAYERS: usize = 8;
const ARENA_START_TIMER_SECONDS: i64 = 1;
const ARENA_WIN_TIMEOUT_SECONDS: i64 = 5;
const LIGHTCYCLE_SPEED: f64 = 55.0;
const LIGHTCYCLE_BOOST_SPEED: f64 = 110.0;
const SLIPSTREAM_DISTANCE: f64 = 15.0;
const SLIPSTREAM_RAMP_UP_SECONDS: f64 = 5.0;
const SLIPSTREAM_RAMP_DOWN_SECONDS: f64 = 2.0;

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
    pub npc_states: HashMap<PlayerId, npc::NpcState>,

    #[serde(skip)]
    pub winner_at: Option<DateTime<Utc>>,

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

        // run NPC AI — extract npc_states so we can mutate it while reading arena immutably
        let mut npc_states = mem::take(&mut self.npc_states);
        let npc_updates = npc::update_npc_inputs(self, &mut npc_states, delta_time);
        self.npc_states = npc_states;
        self.updates.extend(npc_updates);
        self.apply_updates();

        self.update_lightcycle_speeds(delta_time)
            .apply_updates()
            .update_lightcycle_positions(delta_time)
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

    fn update_lightcycle_speeds(&mut self, delta_time: f64) -> &mut Self {
        let speed_range = LIGHTCYCLE_BOOST_SPEED - LIGHTCYCLE_SPEED;
        let ramp_up_rate = speed_range / SLIPSTREAM_RAMP_UP_SECONDS;
        let ramp_down_rate = speed_range / SLIPSTREAM_RAMP_DOWN_SECONDS;

        let bike_data: Vec<(PlayerId, ArenaPoint, Direction, f64)> = self
            .lightcycles
            .iter()
            .filter(|(_, lc)| !lc.dead)
            .map(|(id, lc)| (*id, lc.position, lc.direction, lc.speed))
            .collect();

        let mut speed_updates = Vec::new();
        for (id, pos, dir, current_speed) in &bike_data {
            let new_speed = if self.is_slipstreaming(*id, *pos, *dir) {
                (current_speed + ramp_up_rate * delta_time).min(LIGHTCYCLE_BOOST_SPEED)
            } else {
                (current_speed - ramp_down_rate * delta_time).max(LIGHTCYCLE_SPEED)
            };

            if (*current_speed - new_speed).abs() > f64::EPSILON {
                speed_updates.push(ArenaUpdate::UpdateLightcycleSpeed(*id, new_speed));
            }
        }

        self.updates.extend(speed_updates);
        self
    }

    fn is_slipstreaming(&self, bike_id: PlayerId, pos: ArenaPoint, dir: Direction) -> bool {
        let is_vertical = matches!(dir, Direction::Up | Direction::Down);

        for (ribbon_id, ribbon) in self.lightribbons.iter() {
            if *ribbon_id == bike_id {
                continue;
            }

            for segment in ribbon.points.windows(2) {
                if segment_is_parallel_slipstream(pos, is_vertical, segment[0], segment[1]) {
                    return true;
                }
            }

            // Check live segment (ribbon tip to other bike's current position)
            if let Some(other_cycle) = self.lightcycles.get(ribbon_id) {
                if !other_cycle.dead {
                    if let Some(last_point) = ribbon.points.last() {
                        if segment_is_parallel_slipstream(
                            pos,
                            is_vertical,
                            *last_point,
                            other_cycle.position,
                        ) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

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
            for (ribbon_id, lightribbon) in self.lightribbons.iter() {
                // For the bike's own ribbon, skip the last 2 segments (the active
                // segment collinear with the bike's direction and the perpendicular
                // segment sharing the turn-point vertex). At higher speeds the
                // longer travelled segment can falsely intersect these.
                let check_points = if ribbon_id == id {
                    &lightribbon.points[..lightribbon.points.len().saturating_sub(2)]
                } else {
                    &lightribbon.points[..]
                };

                for line in check_points.windows(2) {
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

                // Check the "live" segment for other bikes: the ribbon's last point
                // hasn't been updated yet this frame, so there's a gap between the
                // ribbon tip and the bike's current position. Without this check,
                // collisions in that gap are missed.
                if ribbon_id != id {
                    if let Some(other_cycle) = self.lightcycles.get(ribbon_id) {
                        if !other_cycle.dead {
                            if let Some(last_point) = lightribbon.points.last() {
                                let live_segment = ArenaLine {
                                    from: last_point.to_untyped(),
                                    to: other_cycle.position.to_untyped(),
                                };

                                if travelled.overlaps_segment(&live_segment) {
                                    self.updates
                                        .push(ArenaUpdate::UpdateLightcycleApplyDeath(*id));
                                    continue 'next_lightcycle;
                                }

                                if let Some(intersection) =
                                    travelled.intersection(&live_segment)
                                {
                                    self.updates.push(
                                        ArenaUpdate::UpdateLightcyclePosition(
                                            *id,
                                            ArenaPoint::from_untyped(intersection),
                                        ),
                                    );
                                    self.updates
                                        .push(ArenaUpdate::UpdateLightcycleApplyDeath(*id));
                                    continue 'next_lightcycle;
                                }
                            }
                        }
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
            self.winner_at = Some(Utc::now());
        }

        self
    }

    fn test_round_end(&mut self) -> &mut Self {
        if self.lightcycles.values().all(|lightcycle| lightcycle.dead) {
            self.updates.push(ArenaUpdate::End);
            self.winner_at = None;
        } else if let Some(winner_at) = self.winner_at {
            if Utc::now() >= winner_at + OldDuration::seconds(ARENA_WIN_TIMEOUT_SECONDS) {
                self.updates.push(ArenaUpdate::End);
                self.winner_at = None;
            }
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

            npc_states: Default::default(),

            winner_at: None,

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

/// Returns true if the bike at `pos` (traveling vertically if `bike_is_vertical`,
/// horizontally otherwise) is within `SLIPSTREAM_DISTANCE` of the parallel segment
/// from `p1` to `p2` and alongside it.
fn segment_is_parallel_slipstream(
    pos: ArenaPoint,
    bike_is_vertical: bool,
    p1: ArenaPoint,
    p2: ArenaPoint,
) -> bool {
    if bike_is_vertical {
        // Segment must be vertical (same x coordinate)
        if (p1.x - p2.x).abs() > f64::EPSILON {
            return false;
        }
        let min_y = p1.y.min(p2.y);
        let max_y = p1.y.max(p2.y);
        if (max_y - min_y) <= f64::EPSILON {
            return false;
        }
        (pos.x - p1.x).abs() <= SLIPSTREAM_DISTANCE && pos.y >= min_y && pos.y <= max_y
    } else {
        // Segment must be horizontal (same y coordinate)
        if (p1.y - p2.y).abs() > f64::EPSILON {
            return false;
        }
        let min_x = p1.x.min(p2.x);
        let max_x = p1.x.max(p2.x);
        if (max_x - min_x) <= f64::EPSILON {
            return false;
        }
        (pos.y - p1.y).abs() <= SLIPSTREAM_DISTANCE && pos.x >= min_x && pos.x <= max_x
    }
}
