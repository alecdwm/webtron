mod entities;
mod input;
pub mod npc;
mod updates;
mod util;

use chrono::{DateTime, Duration as OldDuration, Utc};
use getrandom::SysRng;
use log::{error, trace};
use rand_core::{Rng, UnwrapErr};
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
const ARENA_START_TIMER_SECONDS: i64 = 3;
const ARENA_WIN_TIMEOUT_SECONDS: i64 = 3;
const LIGHTCYCLE_SPEED: f64 = 55.0;
const LIGHTCYCLE_BOOST_SPEED: f64 = 110.0;
const SLIPSTREAM_DISTANCE: f64 = 15.0;
const SLIPSTREAM_RAMP_UP_SECONDS: f64 = 5.0;
const SLIPSTREAM_RAMP_DOWN_SECONDS: f64 = 2.0;
/// How far to the side a lightcycle's own trail must be to slipstream it.
const OWN_SLIPSTREAM_MIN_DISTANCE: f64 = 1.0;
const HOLE_RADIUS: f64 = 20.0;
const DEAD_LIGHTRIBBON_SECONDS: i64 = 6;

/// What a lightcycle crashed into.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum DeathCause {
    /// The trail of this player, which may be the lightcycle's own.
    Trail(PlayerId),
    Boundary,
    /// This player's lightcycle, head-on.
    Lightcycle(PlayerId),
}

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
    pub holes: Vec<Hole>,

    #[serde(skip)]
    pub npc_states: HashMap<PlayerId, npc::NpcState>,

    #[serde(skip)]
    pub winner_at: Option<DateTime<Utc>>,
    /// A winner the server hasn't recorded on the leaderboard yet.
    #[serde(skip)]
    pub unrecorded_winner: Option<PlayerId>,
    /// Lightcycles that died, which the server hasn't recorded on the
    /// leaderboard yet. Added to as their deaths are applied.
    #[serde(skip)]
    pub unrecorded_deaths: Vec<(PlayerId, DeathCause)>,
    /// Human players who started a round, which the server hasn't recorded
    /// on the leaderboard yet.
    #[serde(skip)]
    pub unrecorded_round_players: Vec<PlayerId>,

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

    pub fn human_player_ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.players
            .keys()
            .copied()
            .filter(|id| !self.npc_states.contains_key(id))
    }

    /// Whether any human player's lightcycle is still in the round.
    pub fn any_human_alive(&self) -> bool {
        self.lightcycles
            .iter()
            .any(|(id, lightcycle)| !lightcycle.dead && !self.npc_states.contains_key(id))
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
            .create_holes()
            .apply_updates()
            .derez_dead_lightribbons()
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
            let own = *ribbon_id == bike_id;

            for (from, to) in ribbon.segments() {
                // A lightcycle is always in line with the segment it is
                // laying, so its own trail only counts when off to the side.
                let lateral = if is_vertical {
                    (pos.x - from.x).abs()
                } else {
                    (pos.y - from.y).abs()
                };
                if own && lateral < OWN_SLIPSTREAM_MIN_DISTANCE {
                    continue;
                }

                if segment_is_parallel_slipstream(pos, is_vertical, from, to) {
                    return true;
                }
            }

            // Check live segment (ribbon tip to other bike's current position)
            if own {
                continue;
            }
            if let Some(other_cycle) = self.lightcycles.get(ribbon_id) {
                if !other_cycle.dead {
                    if let Some(last_point) = ribbon.last_point() {
                        if segment_is_parallel_slipstream(
                            pos,
                            is_vertical,
                            last_point,
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
                let skip = if ribbon_id == id { 2 } else { 0 };

                for (from, to) in lightribbon.segments_except_newest(skip) {
                    let line = ArenaLine {
                        from: from.to_untyped(),
                        to: to.to_untyped(),
                    };

                    if travelled.overlaps_segment(&line) {
                        self.updates.push(ArenaUpdate::UpdateLightcycleApplyDeath(
                            *id,
                            DeathCause::Trail(*ribbon_id),
                        ));
                        continue 'next_lightcycle;
                    }

                    if let Some(intersection) = travelled.intersection(&line) {
                        self.updates.push(ArenaUpdate::UpdateLightcyclePosition(
                            *id,
                            ArenaPoint::from_untyped(intersection),
                        ));
                        self.updates.push(ArenaUpdate::UpdateLightcycleApplyDeath(
                            *id,
                            DeathCause::Trail(*ribbon_id),
                        ));
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
                            if let Some(last_point) = lightribbon.last_point() {
                                let live_segment = ArenaLine {
                                    from: last_point.to_untyped(),
                                    to: other_cycle.position.to_untyped(),
                                };

                                if travelled.overlaps_segment(&live_segment) {
                                    self.updates.push(ArenaUpdate::UpdateLightcycleApplyDeath(
                                        *id,
                                        DeathCause::Trail(*ribbon_id),
                                    ));
                                    continue 'next_lightcycle;
                                }

                                if let Some(intersection) = travelled.intersection(&live_segment) {
                                    self.updates.push(ArenaUpdate::UpdateLightcyclePosition(
                                        *id,
                                        ArenaPoint::from_untyped(intersection),
                                    ));
                                    self.updates.push(ArenaUpdate::UpdateLightcycleApplyDeath(
                                        *id,
                                        DeathCause::Trail(*ribbon_id),
                                    ));
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
                self.updates.push(ArenaUpdate::UpdateLightcycleApplyDeath(
                    *id,
                    DeathCause::Boundary,
                ));
                continue 'next_lightcycle;
            }

            // test for collisions with other lightcycles
            if let Some((other_id, _)) = self
                .lightcycles
                .iter()
                .filter(|(_, other_lightcycle)| !other_lightcycle.dead)
                .filter(|(other_id, _)| id != *other_id)
                .find(|(_, other_lightcycle)| lightcycle.position == other_lightcycle.position)
            {
                self.updates.push(ArenaUpdate::UpdateLightcycleApplyDeath(
                    *id,
                    DeathCause::Lightcycle(*other_id),
                ));
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

    /// Blows a hole in every trail around each lightcycle that died this update.
    fn create_holes(&mut self) -> &mut Self {
        let new_holes: Vec<Hole> = self
            .lightcycles
            .iter()
            .filter(|(id, lightcycle)| {
                lightcycle.dead && !self.holes.iter().any(|hole| hole.player_id == **id)
            })
            .map(|(id, lightcycle)| Hole {
                player_id: *id,
                position: lightcycle.position,
                radius: HOLE_RADIUS,
                created_at: Utc::now(),
            })
            .collect();

        if new_holes.is_empty() {
            return self;
        }

        for (id, lightribbon) in self.lightribbons.iter() {
            let live_end = self
                .lightcycles
                .get(id)
                .filter(|lightcycle| !lightcycle.dead)
                .map(|lightcycle| lightcycle.position);

            let mut cut = lightribbon.clone();
            for hole in &new_holes {
                cut.cut(hole.position, hole.radius, live_end);
            }

            if cut != *lightribbon {
                self.updates.push(ArenaUpdate::AddLightribbon(*id, cut));
            }
        }

        self.updates
            .extend(new_holes.into_iter().map(ArenaUpdate::AddHole));
        self
    }

    fn derez_dead_lightribbons(&mut self) -> &mut Self {
        let now = Utc::now();
        for hole in &self.holes {
            if now >= hole.created_at + OldDuration::seconds(DEAD_LIGHTRIBBON_SECONDS)
                && self.lightribbons.contains_key(&hole.player_id)
            {
                self.updates
                    .push(ArenaUpdate::DerezLightribbon(hole.player_id));
            }
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
            self.unrecorded_winner = Some(*player_id);
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
            holes: Default::default(),

            npc_states: Default::default(),

            winner_at: None,
            unrecorded_winner: None,
            unrecorded_deaths: Default::default(),
            unrecorded_round_players: Default::default(),

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crashing_into_your_own_trail_is_blamed_on_you() {
        let mut arena = Arena::default();
        let id = PlayerId::default();
        // Heading left into the first leg of its own trail, at x = 90.
        arena.lightcycles.insert(
            id,
            Lightcycle {
                position: ArenaPoint::new(88.0, 150.0),
                direction: Direction::Left,
                ..Default::default()
            },
        );
        arena.lightribbons.insert(
            id,
            Lightribbon {
                paths: vec![vec![
                    ArenaPoint::new(90.0, 100.0),
                    ArenaPoint::new(90.0, 200.0),
                    ArenaPoint::new(150.0, 200.0),
                    ArenaPoint::new(150.0, 150.0),
                    ArenaPoint::new(91.0, 150.0),
                ]],
            },
        );

        arena.calculate_lightcycle_collisions(0.05).apply_updates();
        assert_eq!(arena.unrecorded_deaths, vec![(id, DeathCause::Trail(id))]);
        assert_eq!(
            arena.lightcycles[&id].death_cause,
            Some(DeathCause::Trail(id))
        );
    }

    #[test]
    fn lightcycles_slipstream_their_own_trail() {
        let mut arena = Arena::default();
        let id = PlayerId::default();
        // Up the arena at x = 100, then a U-turn back down at x = 110.
        let ribbon = Lightribbon {
            paths: vec![vec![
                ArenaPoint::new(100.0, 100.0),
                ArenaPoint::new(100.0, 300.0),
                ArenaPoint::new(110.0, 300.0),
                ArenaPoint::new(110.0, 200.0),
            ]],
        };
        arena.lightribbons.insert(id, ribbon);

        let alongside = ArenaPoint::new(110.0, 200.0);
        assert!(arena.is_slipstreaming(id, alongside, Direction::Down));

        // On its own, a straight trail doesn't give a boost.
        let straight = Lightribbon {
            paths: vec![vec![
                ArenaPoint::new(100.0, 100.0),
                ArenaPoint::new(100.0, 300.0),
            ]],
        };
        arena.lightribbons.insert(id, straight);
        let ahead = ArenaPoint::new(100.0, 300.0);
        assert!(!arena.is_slipstreaming(id, ahead, Direction::Up));
    }

    #[test]
    fn clients_see_npc_players_replaced_each_round() {
        let mut arena = Arena::default();
        let human = Player {
            id: PlayerId::default(),
            name: "A".to_string(),
            ..Default::default()
        };
        arena.add_player(human.clone());
        arena.apply_updates();

        for _ in 0..3 {
            arena.process_input(human.id, ArenaInput::Start);
            arena.apply_updates();
            arena.updates.push(ArenaUpdate::End);
            arena.apply_updates();
            // The human, and this round's NPCs until the next round starts.
            assert_eq!(arena.players.len(), 1 + arena.npc_states.len());
            assert!(!arena.npc_states.is_empty());
        }
        assert_eq!(arena.human_player_ids().collect::<Vec<_>>(), vec![human.id]);

        // Clients build their copy of the arena from the same updates.
        let mut client = Arena::default();
        for update in &arena.updates {
            update.apply(&mut client);
        }
        let mut server_players: Vec<_> = arena.players.keys().collect();
        let mut client_players: Vec<_> = client.players.keys().collect();
        server_players.sort_by_key(|id| id.to_string());
        client_players.sort_by_key(|id| id.to_string());
        assert_eq!(client_players, server_players);
    }

    #[test]
    fn rounds_restart_once_only_npcs_are_left() {
        let mut arena = Arena::default();
        let human = Player {
            id: PlayerId::default(),
            name: "A".to_string(),
            ..Default::default()
        };
        arena.add_player(human.clone());
        arena.apply_updates();
        arena.process_input(human.id, ArenaInput::Start);
        arena.apply_updates();
        let first_start = arena.started;
        let first_npcs: Vec<PlayerId> = arena.npc_states.keys().copied().collect();

        // Refused while the human is alive.
        arena.process_input(human.id, ArenaInput::Start);
        arena.apply_updates();
        assert_eq!(arena.started, first_start);

        arena.updates.push(ArenaUpdate::UpdateLightcycleApplyDeath(
            human.id,
            DeathCause::Boundary,
        ));
        arena.apply_updates();
        arena.winner_at = Some(Utc::now());
        arena.process_input(human.id, ArenaInput::Start);
        arena.apply_updates();

        assert_ne!(arena.started, first_start);
        assert!(arena.winner_at.is_none());
        assert!(!arena.lightcycles[&human.id].dead);
        assert!(first_npcs.iter().all(|id| !arena.players.contains_key(id)));
        // Both rounds count as played by the human, and only by them.
        assert_eq!(arena.unrecorded_round_players, vec![human.id, human.id]);
    }
}
