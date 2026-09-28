mod entities;
mod input;
pub mod npc;
mod round;
mod updates;
mod util;

use chrono::{DateTime, Duration as OldDuration, Utc};
use getrandom::SysRng;
use log::{error, trace};
use rand_core::{Rng, UnwrapErr};
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::mem;

pub use self::entities::*;
pub use self::input::*;
pub use self::round::*;
pub use self::updates::*;
pub use self::util::*;

use crate::server::{
    ArenaId, ArenaPoint, ArenaVector, Direction, Emote, Player, PlayerColor, PlayerId,
};

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
/// The speed a braking lightcycle slows to.
const LIGHTCYCLE_BRAKE_SPEED: f64 = 27.5;
/// How quickly braking slows a lightcycle, and how quickly it gets back up
/// to speed after, in arena units per second per second.
const BRAKE_DECELERATION: f64 = 180.0;
const BRAKE_RECOVERY: f64 = 60.0;
/// How long a full brake lasts, and how long an empty one takes to recharge.
const BRAKE_SECONDS: f64 = 1.5;
const BRAKE_RECHARGE_SECONDS: f64 = 6.0;
/// Lightcycles whose paths cross collide with each other when the second is
/// no further than this from the crossing as the first gets there. Further
/// off, the second rides into the first's trail instead.
const COLLISION_MARGIN: f64 = 1.0;
/// A lightcycle has a near miss when it passes this close to the end of a
/// trail, or turns this close to crashing.
const NEAR_MISS_DISTANCE: f64 = 10.0;
/// A near miss this close is perfect: about one update's travel at the
/// lightcycle's usual speed, so any later and it would have crashed.
const PERFECT_NEAR_MISS_DISTANCE: f64 = 3.0;
/// The shortest time between two near misses of one lightcycle.
const NEAR_MISS_COOLDOWN_MILLISECONDS: i64 = 600;
/// How much of a full brake a near miss gives back.
const NEAR_MISS_BRAKE_REFILL: f64 = 0.3;
/// The shortest time between two emotes from one player.
const EMOTE_COOLDOWN_MILLISECONDS: i64 = 250;
/// Chance, in percent, of an NPC taunting a lightcycle that crashes into its
/// trail, or saying GG when it wins.
const NPC_TAUNT_PERCENT: u32 = 30;
/// The most stretches a straight line is followed through, going through
/// portals and round the edges of an arena that wraps round.
const MAX_STRETCHES: usize = 32;

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
    pub round: Round,

    /// The sides of the round's walled-off cells that lightcycles can crash
    /// into, worked out when the round is set.
    #[serde(skip)]
    pub walls: Vec<(ArenaPoint, ArenaPoint)>,
    /// The ends of each of the round's pairs of portals, worked out when the
    /// round is set.
    #[serde(skip)]
    pub portals: Vec<[(ArenaPoint, ArenaPoint); 2]>,
    /// When each lightcycle last had a near miss.
    #[serde(skip)]
    pub near_miss_at: HashMap<PlayerId, DateTime<Utc>>,
    /// When each player last emoted.
    #[serde(skip)]
    pub emote_at: HashMap<PlayerId, DateTime<Utc>>,

    #[serde(skip)]
    pub npc_states: HashMap<PlayerId, npc::NpcState>,
    /// What NPCs' planning keeps from one update to the next.
    #[serde(skip)]
    pub npc_planner_cache: npc::PlannerCache,
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
    /// Human players who left while they had a lightcycle in the round,
    /// until the next round starts. Their lightcycles ride on without them,
    /// and still count for them on the leaderboard.
    #[serde(skip)]
    pub departed: HashMap<PlayerId, Player>,

    /// Changes not yet sent to clients. The server clears them once they
    /// have been sent.
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

    /// Adds a player straight away, so that the next player to join sees
    /// them when checking whether the arena is full.
    pub fn add_player(&mut self, player: Player) {
        self.updates.push(ArenaUpdate::AddPlayer(player.id, player));
        self.apply_updates();
    }

    /// Removes a player straight away. A human player's lightcycle stays in
    /// the round (see `departed`).
    pub fn remove_player(&mut self, player_id: PlayerId) {
        self.updates.push(ArenaUpdate::RemovePlayer(player_id));
        self.apply_updates();
    }

    pub fn human_player_ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.players
            .keys()
            .copied()
            .filter(|id| !self.npc_states.contains_key(id))
    }

    /// Whether the lightcycle of any human player still in the arena is
    /// still in the round.
    pub fn any_human_alive(&self) -> bool {
        self.lightcycles.iter().any(|(id, lightcycle)| {
            !lightcycle.dead && self.players.contains_key(id) && !self.npc_states.contains_key(id)
        })
    }

    /// Whether the player may emote now, which counts as emoting if so.
    pub fn take_emote_turn(&mut self, player_id: PlayerId) -> bool {
        let now = Utc::now();
        if self
            .emote_at
            .get(&player_id)
            .is_some_and(|at| now < *at + OldDuration::milliseconds(EMOTE_COOLDOWN_MILLISECONDS))
        {
            return false;
        }
        self.emote_at.insert(player_id, now);
        true
    }

    /// Seconds since the round started, or 0 before it has.
    pub fn elapsed(&self) -> f64 {
        self.started
            .map(|started| (Utc::now() - started).num_milliseconds() as f64 / 1000.0)
            .unwrap_or(0.0)
            .max(0.0)
    }

    /// The corners of the part of the arena that is open `ahead` seconds
    /// from now, the lowest first.
    pub fn bounds(&self, ahead: f64) -> (ArenaPoint, ArenaPoint) {
        self.round
            .bounds(self.width, self.height, self.elapsed() + ahead)
    }

    /// The distance from `position` in `direction` to the nearest wall, or
    /// to the edge of `bounds` if given.
    pub fn wall_distance(
        &self,
        position: ArenaPoint,
        direction: Direction,
        bounds: Option<(ArenaPoint, ArenaPoint)>,
    ) -> f64 {
        let mut distance = match bounds {
            Some((min, max)) => match direction {
                Direction::Up => max.y - position.y,
                Direction::Down => position.y - min.y,
                Direction::Right => max.x - position.x,
                Direction::Left => position.x - min.x,
            }
            .max(0.0),
            None => f64::INFINITY,
        };
        let far = position + direction.as_velocity() * (self.width + self.height);
        for (a, b) in &self.walls {
            if let Some(contact) = first_contact(position, far, *a, *b) {
                distance = distance.min((contact - position).length());
            }
        }
        distance
    }

    /// A straight line `length` long from `position` in `direction`, as the
    /// stretches it runs through the arena, each with how far along the line
    /// it starts. It is all one stretch unless it goes into a portal, or off
    /// an edge of an arena that wraps round, and comes out somewhere else.
    pub fn stretches(
        &self,
        position: ArenaPoint,
        direction: Direction,
        length: f64,
    ) -> Vec<(ArenaPoint, ArenaPoint, f64)> {
        let velocity = direction.as_velocity();
        let mut stretches = Vec::new();
        let mut from = position;
        let mut along = 0.0;
        for _ in 1..MAX_STRETCHES {
            let Some((distance, exit)) = self.next_gate(from, direction, length - along) else {
                break;
            };
            stretches.push((from, from + velocity * distance, along));
            along += distance;
            from = exit;
        }
        stretches.push((from, from + velocity * (length - along).max(0.0), along));
        stretches
    }

    /// Where a lightcycle at `position` ends up after going `distance` in
    /// `direction`, through any portals or edges of the arena on the way.
    pub fn advance(&self, position: ArenaPoint, direction: Direction, distance: f64) -> ArenaPoint {
        self.stretches(position, direction, distance)
            .last()
            .map_or(position, |&(_, to, _)| to)
    }

    /// How far from `from` in `direction` the nearest portal or wrapping edge
    /// within `reach` is, and where a lightcycle going into it comes out. A
    /// portal is only gone through across it, not along it.
    ///
    /// A portal is gone through once a lightcycle reaches it, even at the
    /// end of its reach, and not when it starts on it: it has just come out
    /// of the portal, and would otherwise go straight back in. An edge is
    /// only gone off with some reach left, and from on it.
    fn next_gate(
        &self,
        from: ArenaPoint,
        direction: Direction,
        reach: f64,
    ) -> Option<(f64, ArenaPoint)> {
        let mut nearest: Option<(f64, ArenaPoint)> = None;
        let mut consider = |gate: Option<(f64, ArenaPoint)>, portal: bool| {
            if let Some((distance, exit)) = gate
                && (if portal {
                    distance > 0.0 && distance <= reach
                } else {
                    distance < reach
                })
                && nearest.is_none_or(|(nearest, _)| distance < nearest)
            {
                nearest = Some((distance, exit));
            }
        };

        if self.round.has(Mutator::Wrap) {
            let (width, height) = (self.width, self.height);
            consider(
                Some(match direction {
                    Direction::Up => ((height - from.y).max(0.0), ArenaPoint::new(from.x, 0.0)),
                    Direction::Down => (from.y.max(0.0), ArenaPoint::new(from.x, height)),
                    Direction::Right => ((width - from.x).max(0.0), ArenaPoint::new(0.0, from.y)),
                    Direction::Left => (from.x.max(0.0), ArenaPoint::new(width, from.y)),
                }),
                false,
            );
        }

        for &[a, b] in &self.portals {
            for ((min, max), (out_min, out_max)) in [(a, b), (b, a)] {
                // The same place on the twin, relative to its corner.
                let across = |x: f64| x - min.x + out_min.x;
                let up = |y: f64| y - min.y + out_min.y;
                let beside_x = min.x < from.x && from.x < max.x;
                let beside_y = min.y < from.y && from.y < max.y;
                let gate = match direction {
                    Direction::Right => (beside_y && from.x <= min.x)
                        .then(|| (min.x - from.x, ArenaPoint::new(out_max.x, up(from.y)))),
                    Direction::Left => (beside_y && from.x >= max.x)
                        .then(|| (from.x - max.x, ArenaPoint::new(out_min.x, up(from.y)))),
                    Direction::Up => (beside_x && from.y <= min.y)
                        .then(|| (min.y - from.y, ArenaPoint::new(across(from.x), out_max.y))),
                    Direction::Down => (beside_x && from.y >= max.y)
                        .then(|| (from.y - max.y, ArenaPoint::new(across(from.x), out_min.y))),
                };
                consider(gate, true);
            }
        }

        nearest
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

        // run NPC AI — extract npc_states and the planner cache so we can mutate them while
        // reading arena immutably
        let mut npc_states = mem::take(&mut self.npc_states);
        let mut planner_cache = mem::take(&mut self.npc_planner_cache);
        let npc_updates =
            npc::update_npc_inputs(self, &mut npc_states, &mut planner_cache, delta_time);
        self.npc_planner_cache = planner_cache;
        let emotes: Vec<(PlayerId, Emote)> = npc_states
            .iter_mut()
            .filter_map(|(id, state)| Some((*id, state.emote.take()?)))
            .collect();
        self.npc_states = npc_states;
        self.updates.extend(npc_updates);
        for (id, emote) in emotes {
            self.npc_emote(id, emote);
        }
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
            .trim_lightribbons()
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

    /// Turns each living lightcycle with the oldest of its queued turns. A
    /// turn taken just before crashing is a near miss.
    fn take_queued_turns(&mut self) -> &mut Self {
        let lightcycles = &self.lightcycles;
        self.queued_turns.retain(|id, _| {
            lightcycles
                .get(id)
                .is_some_and(|lightcycle| !lightcycle.dead)
        });

        let taken: Vec<(PlayerId, Direction)> = self
            .queued_turns
            .iter_mut()
            .filter_map(|(id, turns)| Some((*id, turns.pop_front()?)))
            .collect();

        for (id, direction) in taken {
            let lightcycle = self.lightcycles[&id];
            if direction == lightcycle.direction || direction.is_opposite(lightcycle.direction) {
                continue;
            }

            self.updates.push(ArenaUpdate::UpdateLightribbonAppendPoint(
                id,
                lightcycle.position,
            ));
            self.updates
                .push(ArenaUpdate::UpdateLightcycleDirection(id, direction));

            let gap = self.distance_ahead(id, lightcycle.position, lightcycle.direction);
            if gap <= NEAR_MISS_DISTANCE {
                self.near_miss(id, lightcycle.position, gap);
            }
        }
        self
    }

    /// How far a lightcycle at `position` could go in `direction` before
    /// crashing into a trail, a wall, or the edge of the arena.
    fn distance_ahead(&self, id: PlayerId, position: ArenaPoint, direction: Direction) -> f64 {
        let edges = (!self.round.has(Mutator::Wrap)).then(|| self.bounds(0.0));
        let ray = self.stretches(position, direction, self.width + self.height);
        let mut distance = f64::INFINITY;
        for (index, &(from, to, along)) in ray.iter().enumerate() {
            // Anything past where a stretch goes into a portal or off an edge
            // isn't in the way. The last stretch looks as far as it can.
            let reach = if index + 1 == ray.len() {
                f64::INFINITY
            } else {
                (to - from).length()
            };
            let far = from + direction.as_velocity() * (self.width + self.height);
            let mut ahead = self.wall_distance(from, direction, edges);
            for (ribbon_id, lightribbon) in self.lightribbons.iter() {
                let live_segment = self
                    .lightcycles
                    .get(ribbon_id)
                    .filter(|other| *ribbon_id != id && !other.dead)
                    .zip(lightribbon.last_point())
                    .map(|(other, last_point)| (last_point, other.position));
                for (a, b) in lightribbon.segments().chain(live_segment) {
                    if let Some(contact) = first_contact(from, far, a, b) {
                        ahead = ahead.min((contact - from).length());
                    }
                }
            }
            if ahead <= reach {
                distance = distance.min(along + ahead);
            }
        }
        distance
    }

    /// Gives a lightcycle some brake back for a near miss at `point`, `gap`
    /// short of crashing, unless it had one too recently. A perfect near miss
    /// fills the brake right up, and isn't held back by an earlier one.
    fn near_miss(&mut self, id: PlayerId, point: ArenaPoint, gap: f64) {
        let now = Utc::now();
        let perfect = gap <= PERFECT_NEAR_MISS_DISTANCE;
        if !perfect
            && self.near_miss_at.get(&id).is_some_and(|at| {
                now < *at + OldDuration::milliseconds(NEAR_MISS_COOLDOWN_MILLISECONDS)
            })
        {
            return;
        }
        self.near_miss_at.insert(id, now);
        self.updates.push(ArenaUpdate::NearMiss(id, point, perfect));
        if let Some(lightcycle) = self.lightcycles.get(&id) {
            let brake = match perfect {
                true => 1.0,
                false => (lightcycle.brake + NEAR_MISS_BRAKE_REFILL).min(1.0),
            };
            if brake != lightcycle.brake {
                self.updates
                    .push(ArenaUpdate::UpdateLightcycleBrakeMeter(id, brake));
            }
        }
    }

    /// Slows lightcycles that are braking, and speeds up those slipstreaming,
    /// and drains or recharges their brakes.
    fn update_lightcycle_speeds(&mut self, delta_time: f64) -> &mut Self {
        let scale = self.round.speed_scale();
        let speed = LIGHTCYCLE_SPEED * scale;
        let boost_speed = LIGHTCYCLE_BOOST_SPEED * scale;
        let brake_speed = LIGHTCYCLE_BRAKE_SPEED * scale;
        let speed_range = boost_speed - speed;
        let ramp_up_rate = speed_range / SLIPSTREAM_RAMP_UP_SECONDS;
        let ramp_down_rate = speed_range / SLIPSTREAM_RAMP_DOWN_SECONDS;

        let mut updates = Vec::new();
        for (id, lightcycle) in self.lightcycles.iter() {
            if lightcycle.dead {
                continue;
            }
            let current = lightcycle.speed;
            let braking = lightcycle.is_braking();

            let new_speed = if braking {
                (current - BRAKE_DECELERATION * scale * delta_time).max(brake_speed)
            } else if current < speed {
                (current + BRAKE_RECOVERY * scale * delta_time).min(speed)
            } else if self.is_slipstreaming(*id, lightcycle.position, lightcycle.direction) {
                (current + ramp_up_rate * delta_time).min(boost_speed)
            } else {
                (current - ramp_down_rate * delta_time).max(speed)
            };
            if (current - new_speed).abs() > f64::EPSILON {
                updates.push(ArenaUpdate::UpdateLightcycleSpeed(*id, new_speed));
            }

            // It only recharges once let go, so a brake held down after
            // running out doesn't catch on each bit of charge.
            let brake = if lightcycle.braking {
                (lightcycle.brake - delta_time / BRAKE_SECONDS).max(0.0)
            } else {
                (lightcycle.brake + delta_time / BRAKE_RECHARGE_SECONDS).min(1.0)
            };
            if brake != lightcycle.brake {
                updates.push(ArenaUpdate::UpdateLightcycleBrakeMeter(*id, brake));
            }
        }

        self.updates.extend(updates);
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
            if let Some(other_cycle) = self.lightcycles.get(ribbon_id)
                && !other_cycle.dead
                && let Some(last_point) = ribbon.last_point()
                && segment_is_parallel_slipstream(
                    pos,
                    is_vertical,
                    last_point,
                    other_cycle.position,
                )
            {
                return true;
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
    ///
    /// Crashes are settled in the order they happen during the update. Each
    /// lightcycle that crashes is cut short where it stopped, so its trail
    /// can't be in the way of the others any further along.
    ///
    /// Lightcycles that don't crash may have a near miss, and those that ride
    /// off an edge of an arena with `Mutator::Wrap` come back in at the
    /// opposite edge.
    fn calculate_lightcycle_collisions(
        &mut self,
        starts: &HashMap<PlayerId, ArenaPoint>,
    ) -> &mut Self {
        let mut travels: HashMap<PlayerId, Travel> = self
            .lightcycles
            .iter()
            .filter(|(_, lightcycle)| !lightcycle.dead)
            .map(|(id, lightcycle)| {
                let to = lightcycle.position;
                let from = starts.get(id).copied().unwrap_or(to);
                (*id, Travel::new(self, from, to))
            })
            .collect();
        let bounds = (!self.round.has(Mutator::Wrap)).then(|| self.bounds(0.0));

        // The earliest crash can't be undone by any other, which would have
        // to come before it. Settling it cuts that lightcycle's trail short,
        // which can only take crashes away from the others, so the rest are
        // looked for again.
        let mut crashes: HashMap<PlayerId, DeathCause> = HashMap::new();
        loop {
            let earliest = travels
                .iter()
                .filter(|(id, _)| !crashes.contains_key(id))
                .filter_map(|(id, travel)| {
                    travel
                        .legs
                        .iter()
                        .enumerate()
                        .find_map(|(leg, &(from, to))| {
                            let (time, contact, cause) =
                                self.first_crash(*id, leg, from, to, &travels, bounds)?;
                            Some((time, *id, leg, contact, cause))
                        })
                })
                .min_by(|a, b| a.0.total_cmp(&b.0));
            let Some((_, id, leg, contact, cause)) = earliest else {
                break;
            };
            travels.get_mut(&id).unwrap().stop_at(leg, contact);
            crashes.insert(id, cause);
        }

        let mut near_misses = Vec::new();
        let mut emotes = Vec::new();
        for (id, travel) in travels.iter() {
            let crash = crashes.get(id).copied();
            if crash.is_some() || travel.legs.len() > 1 {
                self.updates
                    .push(ArenaUpdate::UpdateLightcyclePosition(*id, travel.end()));
                self.push_gates(*id, travel, travel.legs.len() - 1);
            }

            match crash {
                Some(cause) => {
                    self.updates
                        .push(ArenaUpdate::UpdateLightcycleApplyDeath(*id, cause));
                    emotes.extend(self.npc_death_emotes(*id, cause, &travels));
                }
                None => {
                    if let Some((point, gap)) = self.near_miss_point(*id, travel, &travels) {
                        near_misses.push((*id, point, gap));
                    }
                }
            }
        }

        // The updates so far are applied first, so near misses see the
        // lightcycles' latest brakes.
        self.apply_updates();
        for (id, point, gap) in near_misses {
            self.near_miss(id, point, gap);
        }
        for (id, emote, always) in emotes {
            if always {
                self.npc_emote(id, emote);
            } else {
                self.npc_taunt(id, emote);
            }
        }
        self
    }

    /// What NPCs say about a lightcycle dying of `cause`, and whether they
    /// always say it or only now and then. An NPC laughs at a lightcycle
    /// that crashes into its trail, or says GG if it held a grudge against
    /// it. An NPC killed by a lightcycle it held a grudge against says RIP.
    fn npc_death_emotes(
        &self,
        id: PlayerId,
        cause: DeathCause,
        travels: &HashMap<PlayerId, Travel>,
    ) -> Vec<(PlayerId, Emote, bool)> {
        let grudge = |npc: PlayerId| {
            self.npc_states
                .get(&npc)
                .and_then(|state| state.grudge)
                .map(|(against, _)| against)
        };
        let killer = match cause {
            DeathCause::Trail(owner) | DeathCause::Lightcycle(owner) if owner != id => Some(owner),
            _ => None,
        };

        let mut emotes = Vec::new();
        if let DeathCause::Trail(owner) = cause
            && owner != id
            && self.npc_states.contains_key(&owner)
            && travels.contains_key(&owner)
        {
            if grudge(owner) == Some(id) {
                emotes.push((owner, Emote::Gg, true));
            } else {
                emotes.push((owner, Emote::Lol, false));
            }
        }
        if killer.is_some() && grudge(id) == killer {
            emotes.push((id, Emote::Rip, true));
        }
        emotes
    }

    /// When a lightcycle first crashes on one leg of its travel this update,
    /// as a fraction of the update, and where, and into what.
    fn first_crash(
        &self,
        id: PlayerId,
        leg: usize,
        from: ArenaPoint,
        to: ArenaPoint,
        travels: &HashMap<PlayerId, Travel>,
        bounds: Option<(ArenaPoint, ArenaPoint)>,
    ) -> Option<(f64, ArenaPoint, DeathCause)> {
        let lightcycle = &self.lightcycles[&id];
        let travel = &travels[&id];

        // Coming back in at the edge of the arena, into a walled-off cell.
        if leg > 0 && self.round.is_blocked_at(from, self.width, self.height) {
            return Some((travel.time(leg, from), from, DeathCause::Boundary));
        }

        // Coming out of a portal, or back in at an edge, onto a trail, or
        // onto another lightcycle's travel this update, timed as a crossing.
        // `first_contact` leaves out touching where a leg starts.
        if leg > 0 {
            let time = travel.time(leg, from);
            for (ribbon_id, lightribbon) in self.lightribbons.iter() {
                if lightribbon.segments().any(|(a, b)| touches(from, a, b)) {
                    return Some((time, from, DeathCause::Trail(*ribbon_id)));
                }
                let Some(other_travel) = travels.get(ribbon_id).filter(|_| ribbon_id != &id) else {
                    continue;
                };
                for (other_leg, &(a, b)) in other_travel.legs.iter().enumerate() {
                    if touches(from, a, b)
                        && let Some(cause) =
                            crossing(travel, leg, *ribbon_id, other_travel, other_leg, from)
                    {
                        return Some((time, from, cause));
                    }
                }
            }
        }

        // test for head-on collisions with lightcycles coming the other
        // way along the same line, which meet between where the two were,
        // unless the other crashed before getting there
        let head_on = (leg == 0)
            .then(|| {
                travels
                    .iter()
                    .filter(|(other_id, _)| **other_id != id)
                    .filter_map(|(other_id, other_travel)| {
                        let other = &self.lightcycles[other_id];
                        if !other.direction.is_opposite(lightcycle.direction) {
                            return None;
                        }
                        let (other_from, other_to) = other_travel.legs[0];
                        first_contact(from, to, other_from, other_to)?;
                        let meeting = from.lerp(
                            other_from,
                            lightcycle.speed / (lightcycle.speed + other.speed),
                        );
                        if !touches(meeting, other_from, other_to) {
                            return None;
                        }
                        Some(((meeting - from).length(), meeting, *other_id))
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0))
            })
            .flatten();

        // stop the lightcycle at whichever it touches first: a lightcycle
        // head-on, a trail, a wall, or the edge of the arena
        let mut crash: Option<(f64, ArenaPoint, DeathCause)> =
            head_on.map(|(distance, meeting, other_id)| {
                (distance, meeting, DeathCause::Lightcycle(other_id))
            });
        let mut consider = |contact: ArenaPoint, cause: DeathCause| {
            let distance = (contact - from).length();
            if crash.is_none_or(|(nearest, ..)| distance < nearest) {
                crash = Some((distance, contact, cause));
            }
        };

        for (ribbon_id, lightribbon) in self.lightribbons.iter() {
            // Another lightcycle's ribbon doesn't reach where that
            // lightcycle moved this update until the ribbon positions are
            // updated, so its travel is checked as well. A lightcycle's own
            // travel is the one being tested, and that of one met head-on is
            // where the two meet.
            let other_travel = travels
                .get(ribbon_id)
                .filter(|_| ribbon_id != &id)
                .filter(|_| head_on.is_none_or(|(.., other_id)| other_id != *ribbon_id));

            for (a, b) in lightribbon.segments() {
                if let Some(contact) = first_contact(from, to, a, b) {
                    consider(contact, DeathCause::Trail(*ribbon_id));
                }
            }

            let other_legs = other_travel.map(|travel| travel.legs.as_slice());
            for (other_leg, &(a, b)) in other_legs.unwrap_or_default().iter().enumerate() {
                let Some(contact) = first_contact(from, to, a, b) else {
                    continue;
                };
                // Along the same line, the other lightcycle's trail is in the
                // way: it's going the same way ahead of this one, or coming
                // the other way and stopped before the two could meet.
                if (to - from).dot(b - a) != 0.0 {
                    consider(contact, DeathCause::Trail(*ribbon_id));
                    continue;
                }
                let other_travel = other_travel.unwrap();
                if let Some(cause) =
                    crossing(travel, leg, *ribbon_id, other_travel, other_leg, contact)
                {
                    consider(contact, cause);
                }
            }
        }

        for (a, b) in &self.walls {
            if let Some(contact) = first_contact(from, to, *a, *b) {
                consider(contact, DeathCause::Boundary);
            }
        }

        if let Some((min, max)) = bounds {
            let outside = |point: ArenaPoint| {
                point.x < min.x || point.y < min.y || point.x > max.x || point.y > max.y
            };
            if outside(from) {
                // The walls closed in on the lightcycle.
                consider(from, DeathCause::Boundary);
            } else if outside(to) {
                consider(to.clamp(min, max), DeathCause::Boundary);
            }
        }

        crash.map(|(_, contact, cause)| (travel.time(leg, contact), contact, cause))
    }

    /// Ends a lightcycle's trail where it went into each portal or off the
    /// edge of the arena, and starts a new path where it came out, for the
    /// first `gates` of its travel's legs.
    fn push_gates(&mut self, id: PlayerId, travel: &Travel, gates: usize) {
        for pair in travel.legs.windows(2).take(gates) {
            let (exit, entry) = (pair[0].1, pair[1].0);
            self.updates
                .push(ArenaUpdate::UpdateLightribbonReplaceLatestPoint(id, exit));
            self.updates
                .push(ArenaUpdate::UpdateLightribbonStartPath(id, entry));
        }
    }

    /// Where a lightcycle's travel this update passed closest by the end of a
    /// trail across its path, and how close, if it passed close by any.
    fn near_miss_point(
        &self,
        id: PlayerId,
        travel: &Travel,
        travels: &HashMap<PlayerId, Travel>,
    ) -> Option<(ArenaPoint, f64)> {
        let mut closest: Option<(ArenaPoint, f64)> = None;
        for &(from, to) in &travel.legs {
            for (ribbon_id, lightribbon) in self.lightribbons.iter() {
                let live_segments = travels
                    .get(ribbon_id)
                    .filter(|_| ribbon_id != &id)
                    .map(|travel| travel.legs.as_slice())
                    .unwrap_or_default();
                for (a, b) in lightribbon.segments().chain(live_segments.iter().copied()) {
                    if let Some((point, gap)) = passes_end(from, to, a, b)
                        && closest.is_none_or(|(_, closest_gap)| gap < closest_gap)
                    {
                        closest = Some((point, gap));
                    }
                }
            }
        }
        closest
    }

    /// An NPC emotes, now and then.
    fn npc_taunt(&mut self, id: PlayerId, emote: Emote) {
        if UnwrapErr(SysRng).next_u32() % 100 < NPC_TAUNT_PERCENT {
            self.npc_emote(id, emote);
        }
    }

    /// An NPC emotes, unless it did so too recently.
    fn npc_emote(&mut self, id: PlayerId, emote: Emote) {
        if self.take_emote_turn(id) {
            self.updates.push(ArenaUpdate::Emote(id, emote));
        }
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
    fn trim_lightribbons(&mut self) -> &mut Self {
        if !self.round.has(Mutator::Snake) {
            return self;
        }
        for (id, lightribbon) in self.lightribbons.iter() {
            if self
                .lightcycles
                .get(id)
                .is_none_or(|lightcycle| lightcycle.dead)
            {
                continue;
            }
            let excess = lightribbon.length() - SNAKE_TRAIL_LENGTH;
            if excess > 0.0 {
                self.updates.push(ArenaUpdate::TrimLightribbon(*id, excess));
            }
        }
        self
    }

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

        if self.lightcycles.len() <= 1 {
            return self;
        }

        let mut alive_lightcycles = self
            .lightcycles
            .iter()
            .filter(|(_, lightcycle)| !lightcycle.dead);

        if let (Some((player_id, _)), None) = (alive_lightcycles.next(), alive_lightcycles.next()) {
            let player_id = *player_id;
            self.updates.push(ArenaUpdate::SetWinner(Some(player_id)));
            self.winner_at = Some(Utc::now());
            self.unrecorded_winner = Some(player_id);
            if self.npc_states.contains_key(&player_id) {
                self.npc_taunt(player_id, Emote::Gg);
            }
        }

        self
    }

    fn test_round_end(&mut self) -> &mut Self {
        if self.lightcycles.values().all(|lightcycle| lightcycle.dead) {
            self.updates.push(ArenaUpdate::End);
            self.winner_at = None;
        } else if let Some(winner_at) = self.winner_at
            && Utc::now() >= winner_at + OldDuration::seconds(ARENA_WIN_TIMEOUT_SECONDS)
        {
            self.updates.push(ArenaUpdate::End);
            self.winner_at = None;
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
            round: Default::default(),

            walls: Default::default(),
            portals: Default::default(),
            near_miss_at: Default::default(),
            emote_at: Default::default(),

            npc_states: Default::default(),
            npc_planner_cache: Default::default(),
            queued_turns: Default::default(),

            winner_at: None,
            unrecorded_winner: None,
            unrecorded_deaths: Default::default(),
            unrecorded_round_players: Default::default(),
            departed: Default::default(),

            updates: Default::default(),
            updates_applied_so_far: 0,
        }
    }
}

/// ArenaOverview represents an overview of an Arena for the arena selection screen.
///
/// It lists human players only. NPCs make way for humans when the next round
/// starts, so an arena filled out with NPCs is still open to join.
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
            players: arena
                .human_player_ids()
                .map(|id| (id, arena.players[&id].clone()))
                .collect(),
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

/// What a lightcycle crashes into where its travel crosses that of lightcycle
/// `other_id`, at `point` on leg `leg` of `travel` and leg `other_leg` of
/// `other`. Whichever gets there second rides into the other's trail, unless
/// the two get there together.
fn crossing(
    travel: &Travel,
    leg: usize,
    other_id: PlayerId,
    other: &Travel,
    other_leg: usize,
    point: ArenaPoint,
) -> Option<DeathCause> {
    let mine = travel.time(leg, point);
    let theirs = other.time(other_leg, point);
    let behind = if theirs <= mine {
        (mine - theirs) * travel.length
    } else {
        (theirs - mine) * other.length
    };
    if behind <= COLLISION_MARGIN {
        Some(DeathCause::Lightcycle(other_id))
    } else if theirs < mine {
        Some(DeathCause::Trail(other_id))
    } else {
        None
    }
}

/// Whether `point` is on the horizontal or vertical segment from `a` to `b`.
fn touches(point: ArenaPoint, a: ArenaPoint, b: ArenaPoint) -> bool {
    (a.x.min(b.x)..=a.x.max(b.x)).contains(&point.x)
        && (a.y.min(b.y)..=a.y.max(b.y)).contains(&point.y)
}

/// A lightcycle's travel in one update: a single leg, or one more for each
/// portal it goes into or edge of the arena it wraps round.
struct Travel {
    legs: Vec<(ArenaPoint, ArenaPoint)>,
    /// How far the lightcycle would go in the update without crashing, at
    /// the one speed all the way.
    length: f64,
}

impl Travel {
    fn new(arena: &Arena, from: ArenaPoint, to: ArenaPoint) -> Self {
        let delta = to - from;
        let direction = if delta.x > 0.0 {
            Direction::Right
        } else if delta.x < 0.0 {
            Direction::Left
        } else if delta.y > 0.0 {
            Direction::Up
        } else {
            Direction::Down
        };
        let stretches = arena.stretches(from, direction, delta.length());
        // A single leg ends exactly where the lightcycle is, which its trail
        // is drawn to.
        let legs = if stretches.len() == 1 {
            vec![(from, to)]
        } else {
            stretches
                .into_iter()
                .map(|(from, to, _)| (from, to))
                .collect()
        };
        let length = legs.iter().map(|(from, to)| (*to - *from).length()).sum();
        Self { legs, length }
    }

    /// When the lightcycle is at `point` on leg `leg`, as a fraction of the
    /// update.
    fn time(&self, leg: usize, point: ArenaPoint) -> f64 {
        if self.length == 0.0 {
            return 0.0;
        }
        let before: f64 = self.legs[..leg]
            .iter()
            .map(|(from, to)| (*to - *from).length())
            .sum();
        (before + (point - self.legs[leg].0).length()) / self.length
    }

    /// Cuts the travel short where the lightcycle stopped, at `point` on leg
    /// `leg`.
    fn stop_at(&mut self, leg: usize, point: ArenaPoint) {
        self.legs.truncate(leg + 1);
        self.legs[leg].1 = point;
    }

    /// Where the lightcycle ends up.
    fn end(&self) -> ArenaPoint {
        self.legs[self.legs.len() - 1].1
    }
}

/// Where a lightcycle travelling in a straight line from `from` to `to`
/// crosses the line through the trail segment from `a` to `b`, which is
/// across its path, and how far from the end of the segment, if it does so
/// beyond the end and no more than `NEAR_MISS_DISTANCE` from it. Crossing at
/// `from` doesn't count, as that is where a lightcycle's own trail turns.
fn passes_end(
    from: ArenaPoint,
    to: ArenaPoint,
    a: ArenaPoint,
    b: ArenaPoint,
) -> Option<(ArenaPoint, f64)> {
    // Swapped round for a vertical path, so the path runs along x.
    let flip = |point: ArenaPoint| ArenaPoint::new(point.y, point.x);
    let horizontal = from.y == to.y && from.x != to.x;
    let vertical = from.x == to.x && from.y != to.y;
    let (from, to, a, b) = match (horizontal, vertical) {
        (true, _) => (from, to, a, b),
        (_, true) => (flip(from), flip(to), flip(a), flip(b)),
        _ => return None,
    };
    // The segment must run across the path.
    if a.x != b.x || a.y == b.y {
        return None;
    }
    let crossing = a.x;
    let (low, high) = (from.x.min(to.x), from.x.max(to.x));
    if crossing == from.x || crossing < low || crossing > high {
        return None;
    }
    let gap = (a.y.min(b.y) - from.y).max(from.y - a.y.max(b.y));
    if gap <= 0.0 || gap > NEAR_MISS_DISTANCE {
        return None;
    }
    let point = ArenaPoint::new(crossing, from.y);
    Some((if horizontal { point } else { flip(point) }, gap))
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

    #[test]
    fn lightcycles_meeting_head_on_crash_into_each_other() {
        let id = PlayerId::default();
        let other = PlayerId::default();
        // On the same line, heading for each other 6 apart.
        let mut arena = arena_with_lightcycle(
            id,
            ArenaPoint::new(100.0, 50.0),
            Direction::Right,
            vec![ArenaPoint::new(90.0, 50.0), ArenaPoint::new(100.0, 50.0)],
        );
        arena.lightcycles.insert(
            other,
            Lightcycle {
                position: ArenaPoint::new(106.0, 50.0),
                direction: Direction::Left,
                ..Default::default()
            },
        );
        arena.lightribbons.insert(
            other,
            Lightribbon {
                paths: vec![vec![
                    ArenaPoint::new(116.0, 50.0),
                    ArenaPoint::new(106.0, 50.0),
                ]],
            },
        );

        arena.update(0.05);
        assert!(!arena.lightcycles[&id].dead);
        arena.update(0.05);

        assert_eq!(
            arena.lightcycles[&id].death_cause,
            Some(DeathCause::Lightcycle(other))
        );
        assert_eq!(
            arena.lightcycles[&other].death_cause,
            Some(DeathCause::Lightcycle(id))
        );
        // They meet halfway between where they were.
        for lightcycle in [&arena.lightcycles[&id], &arena.lightcycles[&other]] {
            assert!((lightcycle.position - ArenaPoint::new(103.0, 50.0)).length() < 1e-9);
        }
    }

    /// Lightcycles `a` and `b` each moved this update from `starts` to
    /// `ends` in a straight line, trailing 20 back from the start, with the
    /// collisions calculated.
    fn crossing_paths(ids: [PlayerId; 2], starts: [ArenaPoint; 2], ends: [ArenaPoint; 2]) -> Arena {
        crossing_paths_in(Arena::default(), ids, starts, ends)
    }

    /// As `crossing_paths`, in `arena`.
    fn crossing_paths_in(
        mut arena: Arena,
        [a, b]: [PlayerId; 2],
        starts: [ArenaPoint; 2],
        ends: [ArenaPoint; 2],
    ) -> Arena {
        for ((id, start), end) in [a, b].into_iter().zip(starts).zip(ends) {
            let heading = (end - start).normalize();
            let direction = if heading.x > 0.0 {
                Direction::Right
            } else if heading.x < 0.0 {
                Direction::Left
            } else if heading.y > 0.0 {
                Direction::Up
            } else {
                Direction::Down
            };
            arena.lightcycles.insert(
                id,
                Lightcycle {
                    position: end,
                    direction,
                    ..Default::default()
                },
            );
            arena.lightribbons.insert(
                id,
                Lightribbon {
                    paths: vec![vec![start - heading * 20.0, start]],
                },
            );
        }
        let starts = HashMap::from([(a, starts[0]), (b, starts[1])]);
        arena
            .calculate_lightcycle_collisions(&starts)
            .apply_updates();
        arena
    }

    #[test]
    fn lightcycles_crossing_paths_together_crash_into_each_other() {
        let (a, b) = (PlayerId::default(), PlayerId::default());
        // Both halfway through the update at (101.375, 50), and then b just
        // behind a, 0.5 short of the crossing as a gets there.
        for b_start in [48.625, 48.125] {
            let arena = crossing_paths(
                [a, b],
                [
                    ArenaPoint::new(100.0, 50.0),
                    ArenaPoint::new(101.375, b_start),
                ],
                [
                    ArenaPoint::new(102.75, 50.0),
                    ArenaPoint::new(101.375, b_start + 2.75),
                ],
            );
            assert_eq!(
                arena.lightcycles[&a].death_cause,
                Some(DeathCause::Lightcycle(b))
            );
            assert_eq!(
                arena.lightcycles[&b].death_cause,
                Some(DeathCause::Lightcycle(a))
            );
            for id in [a, b] {
                assert_eq!(
                    arena.lightcycles[&id].position,
                    ArenaPoint::new(101.375, 50.0)
                );
            }
        }
    }

    #[test]
    fn the_second_lightcycle_to_a_crossing_rides_into_the_first_ones_trail() {
        let (a, b) = (PlayerId::default(), PlayerId::default());
        // b gets to (102.5, 50) after 1 of its travel, a after 2.5.
        let arena = crossing_paths(
            [a, b],
            [ArenaPoint::new(100.0, 50.0), ArenaPoint::new(102.5, 49.0)],
            [ArenaPoint::new(102.75, 50.0), ArenaPoint::new(102.5, 51.75)],
        );
        assert_eq!(
            arena.lightcycles[&a].death_cause,
            Some(DeathCause::Trail(b))
        );
        assert_eq!(arena.lightcycles[&a].position, ArenaPoint::new(102.5, 50.0));
        assert!(!arena.lightcycles[&b].dead);
        assert_eq!(
            arena.lightcycles[&b].position,
            ArenaPoint::new(102.5, 51.75)
        );
    }

    #[test]
    fn lightcycles_dont_crash_into_trails_cut_short_by_an_earlier_crash() {
        let (a, b) = (PlayerId::default(), PlayerId::default());
        // a would cross b's path at (102, 50) first, but crashes into a trail
        // at x = 101 before it gets there.
        let wall = PlayerId::default();
        let mut arena = Arena::default();
        arena.lightribbons.insert(
            wall,
            Lightribbon {
                paths: vec![vec![
                    ArenaPoint::new(101.0, 0.0),
                    ArenaPoint::new(101.0, 100.0),
                ]],
            },
        );
        let arena = crossing_paths_in(
            arena,
            [a, b],
            [ArenaPoint::new(100.0, 50.0), ArenaPoint::new(102.0, 47.5)],
            [ArenaPoint::new(102.75, 50.0), ArenaPoint::new(102.0, 50.25)],
        );
        assert_eq!(
            arena.lightcycles[&a].death_cause,
            Some(DeathCause::Trail(wall))
        );
        assert!(!arena.lightcycles[&b].dead);
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
    fn braking_slows_a_lightcycle_until_the_brake_runs_out() {
        let id = PlayerId::default();
        let mut arena = arena_going_up(id);
        arena.process_input(id, ArenaInput::Brake(true));
        for _ in 0..10 {
            arena.update(0.05);
        }
        assert_eq!(arena.lightcycles[&id].speed, LIGHTCYCLE_BRAKE_SPEED);
        for _ in 0..30 {
            arena.update(0.05);
        }
        let lightcycle = arena.lightcycles[&id];
        assert_eq!(lightcycle.brake, 0.0);
        assert!(lightcycle.speed > LIGHTCYCLE_BRAKE_SPEED);

        arena.process_input(id, ArenaInput::Brake(false));
        for _ in 0..20 {
            arena.update(0.05);
        }
        assert_eq!(arena.lightcycles[&id].speed, LIGHTCYCLE_SPEED);
        assert!(arena.lightcycles[&id].brake > 0.0);
    }

    #[test]
    fn lightcycles_wrap_round_the_edges_of_a_wrapping_arena() {
        let id = PlayerId::default();
        let mut arena = arena_with_lightcycle(
            id,
            ArenaPoint::new(ARENA_WIDTH - 1.0, 100.0),
            Direction::Right,
            vec![
                ArenaPoint::new(ARENA_WIDTH - 50.0, 100.0),
                ArenaPoint::new(ARENA_WIDTH - 1.0, 100.0),
            ],
        );
        ArenaUpdate::SetRound(Round::new(Shape::Grid, vec![Mutator::Wrap])).apply(&mut arena);

        arena.update(0.05);
        let lightcycle = arena.lightcycles[&id];
        assert!(!lightcycle.dead);
        assert!((lightcycle.position - ArenaPoint::new(1.75, 100.0)).length() < 1e-9);
        assert_eq!(
            arena.lightribbons[&id].paths,
            vec![
                vec![
                    ArenaPoint::new(ARENA_WIDTH - 50.0, 100.0),
                    ArenaPoint::new(ARENA_WIDTH, 100.0)
                ],
                vec![ArenaPoint::new(0.0, 100.0), lightcycle.position],
            ]
        );
    }

    #[test]
    fn lightcycles_wrapping_round_onto_a_path_are_timed_as_a_crossing() {
        let (a, b) = (PlayerId::default(), PlayerId::default());
        let wrapping = || {
            let mut arena = Arena::default();
            ArenaUpdate::SetRound(Round::new(Shape::Grid, vec![Mutator::Wrap])).apply(&mut arena);
            arena
        };
        // a comes back in at (0, 100) after 1 of its travel, and b gets
        // there along the edge after 2.5.
        let arena = crossing_paths_in(
            wrapping(),
            [a, b],
            [
                ArenaPoint::new(ARENA_WIDTH - 1.0, 100.0),
                ArenaPoint::new(0.0, 97.5),
            ],
            [
                ArenaPoint::new(ARENA_WIDTH + 1.75, 100.0),
                ArenaPoint::new(0.0, 100.25),
            ],
        );
        assert!(!arena.lightcycles[&a].dead);
        assert_eq!(
            arena.lightcycles[&b].death_cause,
            Some(DeathCause::Trail(a))
        );

        // b gets there after 0.5, and a comes back in after 2.
        let arena = crossing_paths_in(
            wrapping(),
            [a, b],
            [
                ArenaPoint::new(ARENA_WIDTH - 2.0, 100.0),
                ArenaPoint::new(0.0, 99.5),
            ],
            [
                ArenaPoint::new(ARENA_WIDTH + 0.75, 100.0),
                ArenaPoint::new(0.0, 102.25),
            ],
        );
        assert_eq!(
            arena.lightcycles[&a].death_cause,
            Some(DeathCause::Trail(b))
        );
        assert_eq!(arena.lightcycles[&a].position, ArenaPoint::new(0.0, 100.0));
        assert!(!arena.lightcycles[&b].dead);
    }

    /// A round on the plain grid with one pair of portals, upright lines
    /// from (80, 80) to (80, 160) and from (400, 440) to (400, 520).
    fn portal_round() -> ArenaUpdate {
        ArenaUpdate::SetRound(Round {
            mutators: vec![Mutator::Portals],
            portals: vec![PortalPair::from([
                Portal {
                    x: 2,
                    y: 2,
                    length: 2,
                    upright: true,
                },
                Portal {
                    x: 10,
                    y: 11,
                    length: 2,
                    upright: true,
                },
            ])],
            ..Default::default()
        })
    }

    #[test]
    fn lightcycles_come_out_of_the_far_side_of_a_portals_twin() {
        let id = PlayerId::default();
        let mut arena = arena_with_lightcycle(
            id,
            ArenaPoint::new(78.0, 100.0),
            Direction::Right,
            vec![ArenaPoint::new(50.0, 100.0), ArenaPoint::new(78.0, 100.0)],
        );
        portal_round().apply(&mut arena);

        arena.update(0.05);
        let lightcycle = arena.lightcycles[&id];
        assert!(!lightcycle.dead);
        assert!((lightcycle.position - ArenaPoint::new(400.75, 460.0)).length() < 1e-9);
        assert_eq!(
            arena.lightribbons[&id].paths,
            vec![
                vec![ArenaPoint::new(50.0, 100.0), ArenaPoint::new(80.0, 100.0)],
                vec![ArenaPoint::new(400.0, 460.0), lightcycle.position],
            ]
        );
    }

    #[test]
    fn lightcycles_crash_into_trails_beyond_a_portal() {
        // A trail 1 past the twin, and one right along it.
        for (x, contact) in [(401.0, 401.0), (400.0, 400.0)] {
            let id = PlayerId::default();
            let other = PlayerId::default();
            let mut arena = arena_with_lightcycle(
                id,
                ArenaPoint::new(79.0, 100.0),
                Direction::Right,
                vec![ArenaPoint::new(50.0, 100.0), ArenaPoint::new(79.0, 100.0)],
            );
            arena.lightribbons.insert(
                other,
                Lightribbon {
                    paths: vec![vec![ArenaPoint::new(x, 400.0), ArenaPoint::new(x, 500.0)]],
                },
            );
            portal_round().apply(&mut arena);

            arena.update(0.05);
            let lightcycle = arena.lightcycles[&id];
            assert_eq!(lightcycle.death_cause, Some(DeathCause::Trail(other)));
            assert_eq!(lightcycle.position, ArenaPoint::new(contact, 460.0));
        }
    }

    #[test]
    fn lightcycles_ride_past_the_side_of_a_portal() {
        let id = PlayerId::default();
        // Along the line of the portal, which isn't going through it.
        let mut arena = arena_with_lightcycle(
            id,
            ArenaPoint::new(78.0, 80.0),
            Direction::Right,
            vec![ArenaPoint::new(50.0, 80.0), ArenaPoint::new(78.0, 80.0)],
        );
        portal_round().apply(&mut arena);

        arena.update(0.05);
        assert_eq!(
            arena.lightcycles[&id].position,
            ArenaPoint::new(80.75, 80.0)
        );
    }

    #[test]
    fn lightcycles_go_through_line_portals_from_either_side_and_only_once() {
        // Vertical lines 80 long at x = 120 and x = 400.
        let line = |x| Portal {
            x,
            y: 2,
            length: 2,
            upright: true,
        };
        // Right through the first, left through the second, and landing
        // exactly on the first.
        for (start, direction, end) in [
            (118.0, Direction::Right, 400.75),
            (401.0, Direction::Left, 118.25),
            (117.25, Direction::Right, 400.0),
        ] {
            let id = PlayerId::default();
            let mut arena = arena_with_lightcycle(
                id,
                ArenaPoint::new(start, 100.0),
                direction,
                vec![ArenaPoint::new(start, 100.0), ArenaPoint::new(start, 100.0)],
            );
            ArenaUpdate::SetRound(Round {
                mutators: vec![Mutator::Portals],
                portals: vec![PortalPair::from([line(3), line(10)])],
                ..Default::default()
            })
            .apply(&mut arena);

            arena.update(0.05);
            let lightcycle = arena.lightcycles[&id];
            assert!(!lightcycle.dead);
            assert!(
                (lightcycle.position.x - end).abs() < 1e-9,
                "{start}: {:?}",
                lightcycle.position
            );
            // Riding on from on the line doesn't go back through.
            arena.update(0.05);
            assert!((arena.lightcycles[&id].position.x - end).abs() > 2.0);
            assert!((arena.lightcycles[&id].position.x - end).abs() < 3.0);
        }
    }

    /// An NPC heading up into the end of another lightcycle's trail across
    /// its path, holding a grudge against that lightcycle or not.
    fn npc_grudge_crash(grudge: bool) -> (Arena, PlayerId, PlayerId) {
        let npc = PlayerId::default();
        let other = PlayerId::default();
        let mut arena = arena_going_up(npc);
        arena.lightcycles.insert(
            other,
            Lightcycle {
                position: ArenaPoint::new(20.0, 500.0),
                direction: Direction::Left,
                ..Default::default()
            },
        );
        arena.lightribbons.insert(
            other,
            Lightribbon {
                paths: vec![vec![
                    ArenaPoint::new(90.0, 111.0),
                    ArenaPoint::new(110.0, 111.0),
                ]],
            },
        );
        let mut state = npc::NpcState::new(npc::Style::Sentinel);
        state.time_remaining = f64::INFINITY;
        if grudge {
            state.grudge = Some((other, 10.0));
        }
        arena.npc_states.insert(npc, state);
        (arena, npc, other)
    }

    #[test]
    fn npcs_killed_by_a_lightcycle_they_hold_a_grudge_against_say_rip() {
        for grudge in [true, false] {
            let (mut arena, npc, other) = npc_grudge_crash(grudge);
            // Too close to turn off.
            let starts = HashMap::from([(npc, ArenaPoint::new(100.0, 110.0))]);
            arena.lightcycles.get_mut(&npc).unwrap().position = ArenaPoint::new(100.0, 112.75);
            arena
                .calculate_lightcycle_collisions(&starts)
                .apply_updates();
            assert_eq!(
                arena.lightcycles[&npc].death_cause,
                Some(DeathCause::Trail(other))
            );
            assert_eq!(
                arena.updates.contains(&ArenaUpdate::Emote(npc, Emote::Rip)),
                grudge
            );
        }
    }

    #[test]
    fn npcs_say_gg_when_a_lightcycle_they_hold_a_grudge_against_hits_their_trail() {
        let (mut arena, npc, other) = npc_grudge_crash(false);
        let state = arena.npc_states.remove(&npc).unwrap();
        // The other way round: the lightcycle going up is the victim.
        arena.npc_states.insert(
            other,
            npc::NpcState {
                grudge: Some((npc, 10.0)),
                ..state
            },
        );
        let starts = HashMap::from([
            (npc, ArenaPoint::new(100.0, 110.0)),
            (other, ArenaPoint::new(20.0, 500.0)),
        ]);
        arena.lightcycles.get_mut(&npc).unwrap().position = ArenaPoint::new(100.0, 112.75);
        arena.lightcycles.get_mut(&other).unwrap().position = ArenaPoint::new(17.25, 500.0);
        arena
            .calculate_lightcycle_collisions(&starts)
            .apply_updates();
        assert!(
            arena
                .updates
                .contains(&ArenaUpdate::Emote(other, Emote::Gg))
        );
    }

    #[test]
    fn lightcycles_crash_into_walled_off_cells() {
        let id = PlayerId::default();
        // Heading up into the block in the middle, which starts at y = 240.
        let mut arena = arena_with_lightcycle(
            id,
            ArenaPoint::new(270.0, 238.0),
            Direction::Up,
            vec![ArenaPoint::new(270.0, 200.0), ArenaPoint::new(270.0, 238.0)],
        );
        ArenaUpdate::SetRound(Round::new(Shape::Core, vec![])).apply(&mut arena);

        arena.update(0.05);
        let lightcycle = arena.lightcycles[&id];
        assert_eq!(lightcycle.death_cause, Some(DeathCause::Boundary));
        assert_eq!(lightcycle.position, ArenaPoint::new(270.0, 240.0));
    }

    #[test]
    fn closing_walls_crash_lightcycles_they_reach() {
        let id = PlayerId::default();
        // Riding along 5 in from the left edge, once the walls have closed in
        // by 8.
        let mut arena = arena_with_lightcycle(
            id,
            ArenaPoint::new(5.0, 100.0),
            Direction::Up,
            vec![ArenaPoint::new(5.0, 90.0), ArenaPoint::new(5.0, 100.0)],
        );
        ArenaUpdate::SetRound(Round::new(Shape::Grid, vec![Mutator::Shrink])).apply(&mut arena);
        arena.started = Some(Utc::now() - OldDuration::seconds(12));

        arena.update(0.05);
        assert_eq!(
            arena.lightcycles[&id].death_cause,
            Some(DeathCause::Boundary)
        );
    }

    #[test]
    fn turning_just_before_a_trail_is_a_near_miss() {
        let id = PlayerId::default();
        let other = PlayerId::default();
        let mut arena = arena_going_up(id);
        arena.lightcycles.get_mut(&id).unwrap().brake = 0.5;
        arena.lightribbons.insert(
            other,
            Lightribbon {
                paths: vec![vec![
                    ArenaPoint::new(50.0, 118.0),
                    ArenaPoint::new(150.0, 118.0),
                ]],
            },
        );

        arena.process_input(id, ArenaInput::Turn(Direction::Left));
        arena.update(0.05);
        assert!(
            arena.updates.iter().any(
                |update| matches!(update, ArenaUpdate::NearMiss(near, _, false) if *near == id)
            )
        );
        assert!(arena.lightcycles[&id].brake > 0.5 + NEAR_MISS_BRAKE_REFILL / 2.0);
    }

    #[test]
    fn turning_right_up_against_a_trail_is_a_perfect_near_miss() {
        let id = PlayerId::default();
        let other = PlayerId::default();
        let mut arena = arena_going_up(id);
        arena.lightcycles.get_mut(&id).unwrap().brake = 0.1;
        // Just after a near miss, which doesn't hold back a perfect one.
        arena.near_miss_at.insert(id, Utc::now());
        arena.lightribbons.insert(
            other,
            Lightribbon {
                paths: vec![vec![
                    ArenaPoint::new(50.0, 112.0),
                    ArenaPoint::new(150.0, 112.0),
                ]],
            },
        );

        arena.process_input(id, ArenaInput::Turn(Direction::Left));
        arena.update(0.05);
        assert!(!arena.lightcycles[&id].dead);
        assert!(
            arena.updates.iter().any(
                |update| matches!(update, ArenaUpdate::NearMiss(near, _, true) if *near == id)
            )
        );
        assert_eq!(arena.lightcycles[&id].brake, 1.0);
    }

    #[test]
    fn passing_close_by_the_end_of_a_trail_is_a_near_miss() {
        let from = ArenaPoint::new(0.0, 0.0);
        let to = ArenaPoint::new(10.0, 0.0);
        // A trail across the path, ending 4 short of it.
        let near = passes_end(
            from,
            to,
            ArenaPoint::new(5.0, 4.0),
            ArenaPoint::new(5.0, 50.0),
        );
        assert_eq!(near, Some((ArenaPoint::new(5.0, 0.0), 4.0)));
        // Too far off, alongside, or behind.
        assert!(
            passes_end(
                from,
                to,
                ArenaPoint::new(5.0, 40.0),
                ArenaPoint::new(5.0, 50.0)
            )
            .is_none()
        );
        assert!(
            passes_end(
                from,
                to,
                ArenaPoint::new(0.0, 4.0),
                ArenaPoint::new(20.0, 4.0)
            )
            .is_none()
        );
        assert!(
            passes_end(
                from,
                to,
                ArenaPoint::new(0.0, 4.0),
                ArenaPoint::new(0.0, 50.0)
            )
            .is_none()
        );
        // The same, for a path going down.
        let near = passes_end(
            ArenaPoint::new(0.0, 10.0),
            ArenaPoint::new(0.0, 0.0),
            ArenaPoint::new(-3.0, 5.0),
            ArenaPoint::new(-30.0, 5.0),
        );
        assert_eq!(near, Some((ArenaPoint::new(0.0, 5.0), 3.0)));
    }

    #[test]
    fn snake_trails_stay_short() {
        let id = PlayerId::default();
        let mut arena = arena_with_lightcycle(
            id,
            ArenaPoint::new(100.0, 500.0),
            Direction::Down,
            vec![
                ArenaPoint::new(100.0, 0.0),
                ArenaPoint::new(100.0, 450.0),
                ArenaPoint::new(50.0, 450.0),
                ArenaPoint::new(50.0, 500.0),
                ArenaPoint::new(100.0, 500.0),
            ],
        );
        ArenaUpdate::SetRound(Round::new(Shape::Grid, vec![Mutator::Snake])).apply(&mut arena);

        arena.update(0.05);
        assert!((arena.lightribbons[&id].length() - SNAKE_TRAIL_LENGTH).abs() < 1e-9);
    }

    #[test]
    fn emotes_are_limited_to_four_a_second() {
        let mut arena = Arena::default();
        let id = PlayerId::default();
        arena.add_player(Player {
            id,
            name: "A".to_string(),
            ..Default::default()
        });
        arena.clear_updates();
        arena.process_input(id, ArenaInput::Emote(Emote::Gg));
        arena.process_input(id, ArenaInput::Emote(Emote::Rip));
        assert_eq!(arena.updates, vec![ArenaUpdate::Emote(id, Emote::Gg)]);
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
            arena.process_input(human.id, ArenaInput::Start(None));
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
    fn scenarios_start_rounds_set_up_as_they_say() {
        let mut arena = Arena::default();
        let human = Player {
            id: PlayerId::default(),
            name: "A".to_string(),
            ..Default::default()
        };
        arena.add_player(human.clone());
        let scenario = Scenario {
            shape: Some(Shape::Pillars),
            mutators: Some(vec![Mutator::Portals]),
            npcs: Some(vec![npc::Style::Baiter; 9]),
        };
        arena.process_input(human.id, ArenaInput::Start(Some(scenario)));
        arena.apply_updates();

        assert_eq!(arena.round.shape, Shape::Pillars);
        assert_eq!(arena.round.mutators, vec![Mutator::Portals]);
        // As many as there is room for alongside the human.
        assert_eq!(arena.npc_states.len(), ARENA_MAX_PLAYERS - 1);
        assert!(
            arena
                .npc_states
                .values()
                .all(|state| state.personality.style == npc::Style::Baiter)
        );
    }

    #[test]
    fn arena_overview_lists_human_players_only() {
        let mut arena = Arena::default();
        let human = Player {
            id: PlayerId::default(),
            name: "A".to_string(),
            ..Default::default()
        };
        arena.add_player(human.clone());
        arena.process_input(human.id, ArenaInput::Start(None));
        arena.apply_updates();
        assert!(!arena.npc_states.is_empty());

        let overview = ArenaOverview::from(&arena);
        assert_eq!(overview.players.keys().collect::<Vec<_>>(), vec![&human.id]);
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
        arena.process_input(human.id, ArenaInput::Start(None));
        arena.apply_updates();
        let first_start = arena.started;
        let first_npcs: Vec<PlayerId> = arena.npc_states.keys().copied().collect();

        // Refused while the human is alive.
        arena.process_input(human.id, ArenaInput::Start(None));
        arena.apply_updates();
        assert_eq!(arena.started, first_start);

        arena.updates.push(ArenaUpdate::UpdateLightcycleApplyDeath(
            human.id,
            DeathCause::Boundary,
        ));
        arena.apply_updates();
        arena.winner_at = Some(Utc::now());
        arena.process_input(human.id, ArenaInput::Start(None));
        arena.apply_updates();

        assert_ne!(arena.started, first_start);
        assert!(arena.winner_at.is_none());
        assert!(!arena.lightcycles[&human.id].dead);
        assert!(first_npcs.iter().all(|id| !arena.players.contains_key(id)));
        // Both rounds count as played by the human, and only by them.
        assert_eq!(arena.unrecorded_round_players, vec![human.id, human.id]);
    }
}
