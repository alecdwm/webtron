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
use std::collections::{HashMap, VecDeque};
use std::mem;

pub use self::entities::*;
pub use self::input::*;
pub use self::updates::*;
pub use self::util::*;

use crate::server::{ArenaId, ArenaPoint, Direction, Player, PlayerColor, PlayerId};

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
/// How many turns a player can have waiting for later updates.
const MAX_QUEUED_TURNS: usize = 3;
/// How long before a round starts a player's turns are accepted, to be taken
/// on its first update.
const EARLY_TURN_MILLISECONDS: i64 = 100;

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
    /// Turns human players have made that are yet to be taken, oldest first.
    /// Each update takes one per lightcycle, so turns made close together
    /// are all kept and each has a segment of trail between them.
    #[serde(skip)]
    pub queued_turns: HashMap<PlayerId, VecDeque<Direction>>,

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

        self.take_queued_turns().apply_updates();

        // run NPC AI — extract npc_states so we can mutate it while reading arena immutably
        let mut npc_states = mem::take(&mut self.npc_states);
        let npc_updates = npc::update_npc_inputs(self, &mut npc_states, delta_time);
        self.npc_states = npc_states;
        self.updates.extend(npc_updates);
        self.apply_updates();

        self.update_lightcycle_speeds(delta_time).apply_updates();

        let starts: HashMap<PlayerId, ArenaPoint> = self
            .lightcycles
            .iter()
            .map(|(id, lightcycle)| (*id, lightcycle.position))
            .collect();

        self.update_lightcycle_positions(delta_time)
            .apply_updates()
            .calculate_lightcycle_collisions(&starts)
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

    /// Turns each living lightcycle with the oldest of its queued turns.
    fn take_queued_turns(&mut self) -> &mut Self {
        let lightcycles = &self.lightcycles;
        self.queued_turns.retain(|id, _| {
            lightcycles
                .get(id)
                .is_some_and(|lightcycle| !lightcycle.dead)
        });

        for (id, turns) in self.queued_turns.iter_mut() {
            let Some(direction) = turns.pop_front() else {
                continue;
            };
            let lightcycle = &self.lightcycles[id];
            if direction == lightcycle.direction || direction.is_opposite(lightcycle.direction) {
                continue;
            }

            self.updates.push(ArenaUpdate::UpdateLightribbonAppendPoint(
                *id,
                lightcycle.position,
            ));
            self.updates
                .push(ArenaUpdate::UpdateLightcycleDirection(*id, direction));
        }
        self
    }

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

    /// Tests each lightcycle's travel this update, from its position in
    /// `starts`. This must be the position exactly as it was, not one worked
    /// out from the speed: the lightcycle's own trail ends there, and a
    /// rounding error would count that as a crash.
    fn calculate_lightcycle_collisions(
        &mut self,
        starts: &HashMap<PlayerId, ArenaPoint>,
    ) -> &mut Self {
        'next_lightcycle: for (id, lightcycle) in self.lightcycles.iter() {
            if lightcycle.dead {
                continue 'next_lightcycle;
            };

            let to = lightcycle.position;
            let from = starts.get(id).copied().unwrap_or(to);

            // test for lightribbon collisions, stopping the lightcycle at the
            // first trail it touches
            let mut crash: Option<(f64, ArenaPoint, PlayerId)> = None;
            for (ribbon_id, lightribbon) in self.lightribbons.iter() {
                // Another lightcycle's ribbon doesn't reach where that
                // lightcycle moved this update until the ribbon positions are
                // updated, so the stretch in between is checked as well. A
                // lightcycle's own stretch is the one it just travelled.
                let live_segment = self
                    .lightcycles
                    .get(ribbon_id)
                    .filter(|other| ribbon_id != id && !other.dead)
                    .zip(lightribbon.last_point())
                    .map(|(other, last_point)| (last_point, other.position));

                for (a, b) in lightribbon.segments().chain(live_segment) {
                    if let Some(contact) = first_contact(from, to, a, b) {
                        let distance = (contact - from).length();
                        if crash.is_none_or(|(nearest, ..)| distance < nearest) {
                            crash = Some((distance, contact, *ribbon_id));
                        }
                    }
                }
            }

            if let Some((_, contact, ribbon_id)) = crash {
                self.updates
                    .push(ArenaUpdate::UpdateLightcyclePosition(*id, contact));
                self.updates.push(ArenaUpdate::UpdateLightcycleApplyDeath(
                    *id,
                    DeathCause::Trail(ribbon_id),
                ));
                continue 'next_lightcycle;
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
            queued_turns: Default::default(),

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

/// Where a lightcycle travelling in a straight line from `from` to `to` first
/// touches the trail segment from `a` to `b`, ends included. Touching it only at
/// `from` doesn't count: a lightcycle's own trail always ends where the
/// lightcycle was, and anything else there would have crashed it already.
///
/// Both the path and the segment must be horizontal or vertical, so that each
/// is the same as its bounding box.
fn first_contact(
    from: ArenaPoint,
    to: ArenaPoint,
    a: ArenaPoint,
    b: ArenaPoint,
) -> Option<ArenaPoint> {
    let min = ArenaPoint::new(
        from.x.min(to.x).max(a.x.min(b.x)),
        from.y.min(to.y).max(a.y.min(b.y)),
    );
    let max = ArenaPoint::new(
        from.x.max(to.x).min(a.x.max(b.x)),
        from.y.max(to.y).min(a.y.max(b.y)),
    );
    if min.x > max.x || min.y > max.y {
        return None;
    }

    // The two touch along the part of the path from `min` to `max`.
    let (near, far) = if (to - from).dot(max - min) >= 0.0 {
        (min, max)
    } else {
        (max, min)
    };
    if far == from {
        return None;
    }
    Some(near)
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

        let starts = HashMap::from([(id, ArenaPoint::new(91.0, 150.0))]);
        arena
            .calculate_lightcycle_collisions(&starts)
            .apply_updates();
        assert_eq!(arena.unrecorded_deaths, vec![(id, DeathCause::Trail(id))]);
        assert_eq!(
            arena.lightcycles[&id].death_cause,
            Some(DeathCause::Trail(id))
        );
    }

    /// A running arena with one lightcycle at `position`, trailing `ribbon`.
    fn arena_with_lightcycle(
        id: PlayerId,
        position: ArenaPoint,
        direction: Direction,
        ribbon: Vec<ArenaPoint>,
    ) -> Arena {
        let mut arena = Arena {
            started: Some(Utc::now() - OldDuration::seconds(1)),
            ..Default::default()
        };
        arena.lightcycles.insert(
            id,
            Lightcycle {
                position,
                direction,
                ..Default::default()
            },
        );
        arena.lightribbons.insert(
            id,
            Lightribbon {
                paths: vec![ribbon],
            },
        );
        arena
    }

    #[test]
    fn crossing_a_trail_exactly_at_one_of_its_points_crashes() {
        let id = PlayerId::default();
        // About to cross its own trail at (100, 155), where the trail has a
        // point between two segments in a straight line. One update moves a
        // lightcycle 2.75 exactly, landing on it.
        let mut arena = arena_with_lightcycle(
            id,
            ArenaPoint::new(102.75, 155.0),
            Direction::Left,
            vec![
                ArenaPoint::new(100.0, 100.0),
                ArenaPoint::new(100.0, 155.0),
                ArenaPoint::new(100.0, 210.0),
                ArenaPoint::new(127.5, 210.0),
                ArenaPoint::new(127.5, 155.0),
                ArenaPoint::new(102.75, 155.0),
            ],
        );

        arena.update(0.05);
        assert_eq!(
            arena.lightcycles[&id].death_cause,
            Some(DeathCause::Trail(id))
        );
        assert_eq!(
            arena.lightcycles[&id].position,
            ArenaPoint::new(100.0, 155.0)
        );
    }

    #[test]
    fn lightcycles_crash_at_the_first_trail_in_their_way() {
        let id = PlayerId::default();
        let other = PlayerId::default();
        // Heading right through its own trail at x = 101 and another's at
        // x = 100, which is inserted second.
        let mut arena = arena_with_lightcycle(
            id,
            ArenaPoint::new(98.0, 50.0),
            Direction::Right,
            vec![
                ArenaPoint::new(101.0, 0.0),
                ArenaPoint::new(101.0, 100.0),
                ArenaPoint::new(90.0, 100.0),
                ArenaPoint::new(90.0, 50.0),
                ArenaPoint::new(98.0, 50.0),
            ],
        );
        arena.lightribbons.insert(
            other,
            Lightribbon {
                paths: vec![vec![
                    ArenaPoint::new(100.0, 0.0),
                    ArenaPoint::new(100.0, 100.0),
                ]],
            },
        );

        arena.update(0.05);
        assert_eq!(
            arena.lightcycles[&id].death_cause,
            Some(DeathCause::Trail(other))
        );
        assert_eq!(
            arena.lightcycles[&id].position,
            ArenaPoint::new(100.0, 50.0)
        );
    }

    /// A running arena with one lightcycle at (100, 110) going up, and a trail
    /// from (100, 100).
    fn arena_going_up(id: PlayerId) -> Arena {
        arena_with_lightcycle(
            id,
            ArenaPoint::new(100.0, 110.0),
            Direction::Up,
            vec![ArenaPoint::new(100.0, 100.0), ArenaPoint::new(100.0, 110.0)],
        )
    }

    #[test]
    fn turns_in_one_update_are_taken_one_update_apart() {
        let id = PlayerId::default();
        let mut arena = arena_going_up(id);

        // A U-turn, both turns arriving before the next update.
        arena.process_input(id, ArenaInput::Turn(Direction::Left));
        arena.process_input(id, ArenaInput::Turn(Direction::Down));

        arena.update(0.05);
        assert_eq!(arena.lightcycles[&id].direction, Direction::Left);
        arena.update(0.05);
        assert_eq!(arena.lightcycles[&id].direction, Direction::Down);
        for _ in 0..10 {
            arena.update(0.05);
        }
        assert!(!arena.lightcycles[&id].dead);
        assert_eq!(
            arena.lightribbons[&id].paths[0][..3],
            [
                ArenaPoint::new(100.0, 100.0),
                ArenaPoint::new(100.0, 110.0),
                ArenaPoint::new(97.25, 110.0),
            ]
        );
    }

    #[test]
    fn turns_are_checked_against_the_last_queued_turn() {
        let id = PlayerId::default();
        let mut arena = arena_going_up(id);

        arena.process_input(id, ArenaInput::Turn(Direction::Left));
        // Reverses the queued turn.
        arena.process_input(id, ArenaInput::Turn(Direction::Right));
        // The same as the queued turn.
        arena.process_input(id, ArenaInput::Turn(Direction::Left));
        // Back the way the lightcycle is going now, which makes a step.
        arena.process_input(id, ArenaInput::Turn(Direction::Up));
        assert_eq!(arena.queued_turns[&id], [Direction::Left, Direction::Up]);
    }

    #[test]
    fn only_a_few_turns_can_be_queued() {
        let id = PlayerId::default();
        let mut arena = arena_going_up(id);

        for direction in [Direction::Left, Direction::Up].repeat(3) {
            arena.process_input(id, ArenaInput::Turn(direction));
        }
        assert_eq!(arena.queued_turns[&id].len(), MAX_QUEUED_TURNS);
    }

    #[test]
    fn turns_just_before_the_start_are_taken_at_the_start() {
        let id = PlayerId::default();
        let mut arena = arena_going_up(id);
        arena.started = Some(Utc::now() + OldDuration::milliseconds(50));

        arena.process_input(id, ArenaInput::Turn(Direction::Left));
        arena.update(0.05);
        assert_eq!(arena.lightcycles[&id].direction, Direction::Up);

        arena.started = Some(Utc::now());
        arena.update(0.05);
        assert_eq!(arena.lightcycles[&id].direction, Direction::Left);
    }

    #[test]
    fn lightcycles_at_any_speed_dont_crash_into_their_own_trail() {
        let id = PlayerId::default();
        // Speeds between the two limits, as when a slipstream boost ramps up
        // or down, move a lightcycle by amounts that aren't exact.
        for step in 0..=1000 {
            let speed = LIGHTCYCLE_SPEED
                + (LIGHTCYCLE_BOOST_SPEED - LIGHTCYCLE_SPEED) * step as f64 / 1000.0;
            let mut arena = arena_with_lightcycle(
                id,
                ArenaPoint::new(280.0, 280.0),
                Direction::Down,
                vec![ArenaPoint::new(280.0, 300.0), ArenaPoint::new(280.0, 280.0)],
            );
            arena.lightcycles.get_mut(&id).unwrap().speed = speed;
            arena.update(0.05);
            arena.process_input(id, ArenaInput::Turn(Direction::Right));
            for _ in 0..10 {
                arena.update(0.05);
            }
            assert!(
                !arena.lightcycles[&id].dead,
                "crashed starting at speed {speed}"
            );
        }
    }

    #[test]
    fn turning_the_way_a_lightcycle_is_going_does_nothing() {
        let id = PlayerId::default();
        let mut arena = arena_going_up(id);

        arena.process_input(id, ArenaInput::Turn(Direction::Up));
        assert!(arena.updates.is_empty());
        assert!(arena.queued_turns.is_empty());
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
