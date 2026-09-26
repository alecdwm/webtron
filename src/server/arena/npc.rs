mod planner;

use super::*;
use crate::server::ArenaLine;
use getrandom::SysRng;
use rand_core::{Rng, UnwrapErr};

use planner::{Caution, Options, Planner};

/// Every round has at least this many lightcycles, human or NPC.
const MIN_LIGHTCYCLES: usize = 2;
/// How far along their current heading other lightcycles are treated as an obstacle.
const NPC_LOOK_AHEAD_DISTANCE: f64 = 40.0;
const NPC_TRAP_SEGMENT_LENGTH: f64 = 30.0;
const NPC_ZIGZAG_LEG_LENGTH: f64 = 16.0;
const NPC_CUTOFF_COOLDOWN: f64 = 3.0;
const NPC_ATTACK_COOLDOWN: f64 = 2.0;
const NPC_OVERTAKE_BUFFER: f64 = 20.0;
/// How far to the side of a trail an NPC rides when drafting it. More than a
/// planning grid cell, so the trail doesn't block the NPC's own cells.
const NPC_DRAFT_OFFSET: f64 = 7.0;
/// A Territory NPC only turns when a turn gains this much over going straight.
const NPC_TERRITORY_TURN_GAIN: f64 = 1.05;

/// Phase within the CutOff behaviour.
#[derive(Debug, Clone, Copy)]
pub enum CutOffPhase {
    /// Waiting for the right moment to turn. Value is remaining seconds.
    Waiting(f64),
    /// Traveling perpendicular, crossing the slipstreamer's lane. Value is distance traveled.
    Crossing(f64),
}

/// Phase within the SlipstreamAttack behaviour.
#[derive(Debug, Clone, Copy)]
pub enum SlipstreamAttackPhase {
    /// Maneuvering to get into slipstream position behind the target.
    Approaching,
    /// In slipstream position, drafting. Waiting to overtake the target.
    Drafting,
    /// Just passed the target. Traveling forward to build a gap before cutting.
    Overtaking,
    /// Cutting across the target's path. Value is distance traveled in the cut.
    Cutting(f64),
}

const NPC_PLAYER_POOL: &[(&str, PlayerColor)] = &[
    ("ABRAXAS", PlayerColor::Green),
    ("ANON", PlayerColor::White),
    ("BARTOK", PlayerColor::Blue),
    ("BECK", PlayerColor::White),
    ("CASTOR", PlayerColor::White),
    ("CLU", PlayerColor::Orange),
    ("CLU_2.0", PlayerColor::Orange),
    ("CROM", PlayerColor::Blue),
    ("DUMONT", PlayerColor::Purple),
    ("DYSON", PlayerColor::Red),
    ("GEM", PlayerColor::White),
    ("ISO_Q", PlayerColor::White),
    ("JARVIS", PlayerColor::Red),
    ("JET", PlayerColor::White),
    ("MCP", PlayerColor::Orange),
    ("MERCURY", PlayerColor::Blue),
    ("RAM", PlayerColor::Blue),
    ("RINZLER", PlayerColor::Red),
    ("SARK", PlayerColor::Red),
    ("TESLER", PlayerColor::Red),
    ("TRON", PlayerColor::Blue),
    ("YORI", PlayerColor::Blue),
    ("ZUSE", PlayerColor::White),
];
const INCOMPATIBLE_NPC_SETS: &[&[&str]] = &[
    &["CASTOR", "ZUSE"],
    &["CLU", "CLU_2.0"],
    &["RINZLER", "TRON"],
];

/// The broad playing style of an NPC, picked at random for each round.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Style {
    /// Claims space aggressively and heads other lightcycles off.
    Hunter,
    /// Rides alongside trails for speed, then uses it to overtake and cut in.
    Racer,
    /// Weaves, lays traps and turns unexpectedly.
    Trickster,
    /// Keeps to open space and punishes anyone who slipstreams it.
    Sentinel,
}

#[derive(Debug, Clone)]
pub struct Personality {
    pub style: Style,
    /// Distance to an obstacle ahead at which the NPC would rather turn.
    pub clearance: f64,
    /// How much less territory than the best move a behaviour's chosen move
    /// may leave before it is overruled, as a fraction of the best.
    pub tolerance: f64,
    /// Chance, in percent, of trying to cut off a lightcycle slipstreaming it.
    pub cutoff_chance: u32,
    /// Chance, in percent, that a cut-off is timed well.
    pub cutoff_accuracy: u32,
    /// Chance, in percent, of taking an opening to slipstream and overtake
    /// another lightcycle.
    pub attack_chance: u32,
}

impl Personality {
    pub fn new(style: Style) -> Self {
        let (clearance, tolerance, cutoff_chance, cutoff_accuracy, attack_chance) = match style {
            Style::Hunter => (random_between(8.0, 14.0), 0.10, 85, 75, 60),
            Style::Racer => (random_between(6.0, 12.0), 0.15, 70, 65, 90),
            Style::Trickster => (random_between(10.0, 25.0), 0.25, 60, 55, 40),
            Style::Sentinel => (random_between(20.0, 35.0), 0.05, 90, 80, 20),
        };

        Self {
            style,
            clearance,
            tolerance,
            cutoff_chance,
            cutoff_accuracy,
            attack_chance,
        }
    }

    fn random() -> Self {
        const STYLES: [Style; 4] = [
            Style::Hunter,
            Style::Racer,
            Style::Trickster,
            Style::Sentinel,
        ];
        Self::new(STYLES[random_below(STYLES.len() as u32) as usize])
    }

    /// Picks the next behaviour at random, weighted by style.
    fn pick_behaviour(&self, lightcycle: &Lightcycle) -> NpcBehaviour {
        let weights: &[(Plan, u32)] = match self.style {
            Style::Hunter => &[
                (Plan::Territory, 45),
                (Plan::Intercept, 30),
                (Plan::Cruise, 15),
                (Plan::Draft, 10),
            ],
            Style::Racer => &[
                (Plan::Draft, 40),
                (Plan::Cruise, 25),
                (Plan::Intercept, 20),
                (Plan::Territory, 10),
                (Plan::Zigzag, 5),
            ],
            Style::Trickster => &[
                (Plan::Cruise, 20),
                (Plan::Intercept, 20),
                (Plan::Zigzag, 20),
                (Plan::RandomTurn, 15),
                (Plan::BuildTrap, 15),
                (Plan::Territory, 5),
                (Plan::Draft, 5),
            ],
            Style::Sentinel => &[
                (Plan::Cruise, 40),
                (Plan::Territory, 40),
                (Plan::Draft, 15),
                (Plan::BuildTrap, 5),
            ],
        };

        let mut roll = random_below(weights.iter().map(|(_, weight)| weight).sum());
        let plan = weights
            .iter()
            .find(|(_, weight)| {
                if roll < *weight {
                    return true;
                }
                roll -= weight;
                false
            })
            .map_or(Plan::Cruise, |(plan, _)| *plan);

        plan.start(lightcycle)
    }
}

/// The behaviours an NPC picks between when its current one runs out.
#[derive(Debug, Clone, Copy)]
enum Plan {
    Cruise,
    Territory,
    Intercept,
    Draft,
    RandomTurn,
    Zigzag,
    BuildTrap,
}

impl Plan {
    fn start(self, lightcycle: &Lightcycle) -> NpcBehaviour {
        match self {
            Plan::Cruise => NpcBehaviour::Cruise,
            Plan::Territory => NpcBehaviour::Territory,
            Plan::Intercept => NpcBehaviour::Intercept { target_id: None },
            Plan::Draft => NpcBehaviour::Draft,
            Plan::RandomTurn => NpcBehaviour::RandomTurn,
            Plan::Zigzag => NpcBehaviour::Zigzag {
                forward: lightcycle.direction,
                side: random_perpendicular(lightcycle.direction),
                legs_left: 4 + 2 * random_below(3) as u8,
                // Start the first leg straight away.
                travelled: NPC_ZIGZAG_LEG_LENGTH,
            },
            Plan::BuildTrap => NpcBehaviour::BuildTrap {
                step: 0,
                distance_in_step: 0.0,
            },
        }
    }
}

/// NPC behaviour mode. Whatever a behaviour asks for is checked against the
/// space it leaves the NPC (see `Options::vet`), so behaviours only need to
/// say what they want to do, not whether it is safe.
#[derive(Debug, Clone)]
pub enum NpcBehaviour {
    /// Go straight, turning towards the most space when blocked.
    Cruise,
    /// Take whichever move claims the most space before anyone else can.
    Territory,
    /// Head for where a lightcycle will be, to get in front of it.
    Intercept { target_id: Option<PlayerId> },
    /// Move alongside a nearby trail to slipstream it.
    Draft,
    /// Insert one random perpendicular turn, then revert to Cruise.
    RandomTurn,
    /// Weave in a staircase, alternating between `side` and `forward` every
    /// leg, which leaves a trail too jagged to slipstream.
    Zigzag {
        forward: Direction,
        side: Direction,
        legs_left: u8,
        travelled: f64,
    },
    /// Build a notch/bump trap: two turns that create a pocket in the trail.
    /// `step` tracks which segment we're on (0 = first leg, 1 = second leg).
    /// `distance_in_step` accumulates how far we've traveled in the current leg.
    BuildTrap { step: u8, distance_in_step: f64 },
    /// Turn to `toward`, travel `remaining` units, then turn back to `resume`.
    LaneChange {
        toward: Direction,
        resume: Direction,
        remaining: f64,
    },
    /// Cut off a detected slipstreamer by turning in front of them.
    CutOff {
        target_id: PlayerId,
        cut_direction: Direction,
        original_direction: Direction,
        cut_distance: f64,
        phase: CutOffPhase,
    },
    /// Actively slipstream another bike, overtake, then cut them off.
    SlipstreamAttack {
        target_id: PlayerId,
        target_direction: Direction,
        approach_direction: Direction,
        cut_distance: f64,
        phase: SlipstreamAttackPhase,
    },
}

impl NpcBehaviour {
    /// Attacks and moves alongside trails are judged only on whether they
    /// lead into a dead end, because they deliberately enter space that other
    /// lightcycles are contesting.
    fn caution(&self) -> Caution {
        match self {
            NpcBehaviour::LaneChange { .. }
            | NpcBehaviour::CutOff { .. }
            | NpcBehaviour::SlipstreamAttack { .. } => Caution::Bold,
            _ => Caution::Contested,
        }
    }

    /// Whether this is a sequence of planned turns, which no longer makes
    /// sense once one of its moves has been overruled.
    fn is_maneuver(&self) -> bool {
        matches!(
            self,
            NpcBehaviour::Zigzag { .. }
                | NpcBehaviour::BuildTrap { .. }
                | NpcBehaviour::LaneChange { .. }
                | NpcBehaviour::CutOff { .. }
                | NpcBehaviour::SlipstreamAttack { .. }
        )
    }
}

#[derive(Debug, Clone)]
pub struct NpcState {
    pub personality: Personality,
    pub behaviour: NpcBehaviour,
    /// Seconds remaining before switching to a new random behaviour.
    pub time_remaining: f64,
    /// Cooldown before the NPC will react to a new slipstreamer.
    pub cutoff_cooldown: f64,
    /// Cooldown before the NPC will consider a new slipstream attack.
    pub attack_cooldown: f64,
}

impl Default for NpcState {
    /// A fresh NPC with a random personality.
    fn default() -> Self {
        Self {
            personality: Personality::random(),
            behaviour: NpcBehaviour::Cruise,
            time_remaining: random_duration_for(&NpcBehaviour::Cruise),
            cutoff_cooldown: 0.0,
            attack_cooldown: 0.0,
        }
    }
}

impl NpcState {
    fn set_behaviour(&mut self, behaviour: NpcBehaviour) {
        self.time_remaining = random_duration_for(&behaviour);
        self.behaviour = behaviour;
    }

    fn cruise(&mut self) {
        self.set_behaviour(NpcBehaviour::Cruise);
    }

    /// Gives up the current behaviour, holding off from starting the same
    /// kind of attack again straight away.
    fn abort(&mut self) {
        match self.behaviour {
            NpcBehaviour::CutOff { .. } => self.cutoff_cooldown = NPC_CUTOFF_COOLDOWN,
            NpcBehaviour::SlipstreamAttack { .. } => self.attack_cooldown = NPC_ATTACK_COOLDOWN,
            _ => {}
        }
        self.cruise();
    }
}

fn random_duration_for(behaviour: &NpcBehaviour) -> f64 {
    match behaviour {
        NpcBehaviour::Cruise => random_between(1.5, 4.0),
        NpcBehaviour::Territory => random_between(3.0, 6.0),
        NpcBehaviour::Intercept { .. } => random_between(2.0, 4.0),
        NpcBehaviour::Draft => 1.0,
        NpcBehaviour::RandomTurn => 0.5,
        NpcBehaviour::Zigzag { .. } => 6.0,
        NpcBehaviour::BuildTrap { .. } => 5.0,
        NpcBehaviour::LaneChange { .. } => 4.0,
        NpcBehaviour::CutOff { .. } => 10.0,
        NpcBehaviour::SlipstreamAttack { .. } => 15.0,
    }
}

fn random_below(n: u32) -> u32 {
    UnwrapErr(SysRng).next_u32() % n.max(1)
}

fn random_between(min: f64, max: f64) -> f64 {
    min + (max - min) * (UnwrapErr(SysRng).next_u32() as f64 / u32::MAX as f64)
}

fn percent_chance(percent: u32) -> bool {
    random_below(100) < percent
}

fn random_perpendicular(direction: Direction) -> Direction {
    direction.perpendicular_directions()[random_below(2) as usize]
}

pub fn create_npc_players(count: usize) -> Vec<Player> {
    let mut pool_remaining = NPC_PLAYER_POOL.to_vec();
    let mut players = Vec::with_capacity(count);

    for _ in 0..count {
        if pool_remaining.is_empty() {
            break;
        }

        let index = UnwrapErr(SysRng).next_u32() as usize % pool_remaining.len();
        let (name, color) = pool_remaining.remove(index);

        // remove incompatible NPCs from the pool
        for incompatible_set in INCOMPATIBLE_NPC_SETS {
            if incompatible_set.iter().any(|n| *n == name) {
                for incompatible_name in *incompatible_set {
                    if let Some(pos) = pool_remaining
                        .iter()
                        .position(|(n, _)| *n == *incompatible_name)
                    {
                        pool_remaining.remove(pos);
                    }
                }
            }
        }

        players.push(Player {
            id: PlayerId::default(),
            name: name.to_string(),
            color,
            npc: true,
        });
    }

    players
}

/// How many NPCs to add to a round: enough to make a random total number of
/// lightcycles, from `MIN_LIGHTCYCLES` (or the number of human players, if
/// more) up to `ARENA_MAX_PLAYERS`.
pub fn npc_count_for_arena(human_player_count: usize) -> usize {
    let min_total = human_player_count.max(MIN_LIGHTCYCLES);
    if min_total >= ARENA_MAX_PLAYERS {
        return ARENA_MAX_PLAYERS.saturating_sub(human_player_count);
    }
    let total = min_total + random_below((ARENA_MAX_PLAYERS - min_total + 1) as u32) as usize;
    total - human_player_count
}

/// What a behaviour can see when choosing where to go.
struct Context<'a> {
    arena: &'a Arena,
    npc_ids: &'a [PlayerId],
    npc_id: PlayerId,
    lightcycle: &'a Lightcycle,
    options: &'a Options,
    delta_time: f64,
}

/// Run AI for all NPCs, producing turn updates where needed.
///
/// `npc_states` is passed separately (extracted from the arena via `mem::take`) so that we can
/// read the arena immutably while mutating NPC state.
pub fn update_npc_inputs(
    arena: &Arena,
    npc_states: &mut HashMap<PlayerId, NpcState>,
    delta_time: f64,
) -> Vec<ArenaUpdate> {
    let mut updates = Vec::new();

    let npc_ids: Vec<PlayerId> = npc_states.keys().copied().collect();
    let any_alive = npc_ids.iter().any(|id| {
        arena
            .lightcycles
            .get(id)
            .is_some_and(|lightcycle| !lightcycle.dead)
    });
    if !any_alive {
        return updates;
    }

    let planner = Planner::new(arena);

    for (npc_id, state) in npc_states.iter_mut() {
        let lightcycle = match arena.lightcycles.get(npc_id) {
            Some(lc) if !lc.dead => lc,
            _ => continue,
        };

        // Tick the behaviour timer and potentially switch behaviours.
        state.time_remaining -= delta_time;
        if state.time_remaining <= 0.0 {
            state.abort();
            let behaviour = state.personality.pick_behaviour(lightcycle);
            state.set_behaviour(behaviour);
        }

        state.cutoff_cooldown = (state.cutoff_cooldown - delta_time).max(0.0);
        state.attack_cooldown = (state.attack_cooldown - delta_time).max(0.0);
        react_to_slipstreamer(arena, *npc_id, lightcycle, state);
        look_for_slipstream_attack(arena, *npc_id, lightcycle, state);

        let options = planner.options(*npc_id, lightcycle, delta_time);
        let context = Context {
            arena,
            npc_ids: &npc_ids,
            npc_id: *npc_id,
            lightcycle,
            options: &options,
            delta_time,
        };
        let request = choose_direction(&context, state);
        let (direction, overruled) =
            options.vet(request, &state.personality, state.behaviour.caution());
        if overruled && state.behaviour.is_maneuver() {
            state.abort();
        }

        if direction != lightcycle.direction {
            updates.push(ArenaUpdate::UpdateLightribbonAppendPoint(
                *npc_id,
                lightcycle.position,
            ));
            updates.push(ArenaUpdate::UpdateLightcycleDirection(*npc_id, direction));
        }
    }

    updates
}

/// When another lightcycle is slipstreaming this NPC's trail, either plan to
/// cut it off or shift sideways out of its way.
fn react_to_slipstreamer(
    arena: &Arena,
    npc_id: PlayerId,
    lightcycle: &Lightcycle,
    state: &mut NpcState,
) {
    if matches!(state.behaviour, NpcBehaviour::CutOff { .. }) || state.cutoff_cooldown > 0.0 {
        return;
    }

    let Some((target_id, target_pos, target_speed)) =
        find_slipstreamer_on_trail(arena, npc_id, lightcycle)
    else {
        return;
    };

    // Direction toward the slipstreamer.
    let perp = lightcycle.direction.perpendicular_directions();
    let to_target = target_pos - lightcycle.position;
    let cut_direction =
        if perp[0].as_velocity().dot(to_target) > perp[1].as_velocity().dot(to_target) {
            perp[0]
        } else {
            perp[1]
        };

    let personality = &state.personality;
    if !percent_chance(personality.cutoff_chance) {
        state.cutoff_cooldown = NPC_CUTOFF_COOLDOWN;
        if percent_chance(50) {
            let away = if cut_direction == perp[0] {
                perp[1]
            } else {
                perp[0]
            };
            state.set_behaviour(NpcBehaviour::LaneChange {
                toward: away,
                resume: lightcycle.direction,
                remaining: random_between(20.0, 35.0),
            });
        }
        return;
    }

    let npc_vel = lightcycle.direction.as_velocity();
    let delta = lightcycle.position - target_pos;
    let distance_behind = npc_vel.dot(delta);
    let perp_distance = match lightcycle.direction {
        Direction::Up | Direction::Down => (target_pos.x - lightcycle.position.x).abs(),
        Direction::Left | Direction::Right => (target_pos.y - lightcycle.position.y).abs(),
    };

    let closing_speed = target_speed - lightcycle.speed;
    if closing_speed <= 1.0 || distance_behind <= 5.0 {
        return;
    }

    let catch_up_time = distance_behind / closing_speed;
    let wait_time = if percent_chance(personality.cutoff_accuracy) {
        catch_up_time * random_between(0.5, 0.75)
    } else {
        // Too late: the slipstreamer is level by the time the NPC turns.
        catch_up_time * random_between(1.1, 1.6)
    };

    state.set_behaviour(NpcBehaviour::CutOff {
        target_id,
        cut_direction,
        original_direction: lightcycle.direction,
        cut_distance: perp_distance + 3.0,
        phase: CutOffPhase::Waiting(wait_time.max(0.1)),
    });
}

/// Start slipstreaming another lightcycle when one is well placed for it.
fn look_for_slipstream_attack(
    arena: &Arena,
    npc_id: PlayerId,
    lightcycle: &Lightcycle,
    state: &mut NpcState,
) {
    if state.attack_cooldown > 0.0
        || matches!(
            state.behaviour,
            NpcBehaviour::CutOff { .. } | NpcBehaviour::SlipstreamAttack { .. }
        )
    {
        return;
    }

    match find_slipstream_attack_target(arena, npc_id, lightcycle) {
        Some((target_id, target_direction, approach_direction, cut_distance))
            if percent_chance(state.personality.attack_chance) =>
        {
            state.set_behaviour(NpcBehaviour::SlipstreamAttack {
                target_id,
                target_direction,
                approach_direction,
                cut_distance,
                phase: SlipstreamAttackPhase::Approaching,
            });
        }
        _ => state.attack_cooldown = NPC_ATTACK_COOLDOWN,
    }
}

/// Returns the direction the current behaviour wants, or `None` to carry on
/// straight ahead.
fn choose_direction(context: &Context, state: &mut NpcState) -> Option<Direction> {
    let lightcycle = context.lightcycle;

    match state.behaviour.clone() {
        NpcBehaviour::Cruise => None,
        NpcBehaviour::Territory => choose_territory(context.options),
        NpcBehaviour::Intercept { target_id } => choose_intercept(context, target_id, state),
        NpcBehaviour::Draft => {
            match find_draft_lane(context) {
                Some((toward, distance)) => state.set_behaviour(NpcBehaviour::LaneChange {
                    toward,
                    resume: lightcycle.direction,
                    remaining: distance,
                }),
                None => state.cruise(),
            }
            choose_direction(context, state)
        }
        NpcBehaviour::RandomTurn => {
            state.cruise();
            Some(random_perpendicular(lightcycle.direction))
        }
        NpcBehaviour::Zigzag {
            forward,
            side,
            legs_left,
            travelled,
        } => {
            let travelled = travelled + lightcycle.speed * context.delta_time;
            if travelled < NPC_ZIGZAG_LEG_LENGTH {
                state.behaviour = NpcBehaviour::Zigzag {
                    forward,
                    side,
                    legs_left,
                    travelled,
                };
                return None;
            }
            if legs_left == 0 {
                state.cruise();
                return None;
            }
            state.behaviour = NpcBehaviour::Zigzag {
                forward,
                side,
                legs_left: legs_left - 1,
                travelled: 0.0,
            };
            Some(if lightcycle.direction == side {
                forward
            } else {
                side
            })
        }
        NpcBehaviour::BuildTrap {
            step,
            distance_in_step,
        } => choose_trap(context, step, distance_in_step, state),
        NpcBehaviour::LaneChange {
            toward,
            resume,
            remaining,
        } => {
            if lightcycle.direction != toward {
                return Some(toward);
            }
            let remaining = remaining - lightcycle.speed * context.delta_time;
            if remaining > 0.0 {
                state.behaviour = NpcBehaviour::LaneChange {
                    toward,
                    resume,
                    remaining,
                };
                return None;
            }
            state.cruise();
            Some(resume)
        }
        NpcBehaviour::CutOff {
            target_id,
            cut_direction,
            original_direction,
            cut_distance,
            phase,
        } => choose_cutoff(
            context,
            target_id,
            cut_direction,
            original_direction,
            cut_distance,
            phase,
            state,
        ),
        NpcBehaviour::SlipstreamAttack {
            target_id,
            target_direction,
            approach_direction,
            cut_distance,
            phase,
        } => choose_slipstream_attack(
            context,
            target_id,
            target_direction,
            approach_direction,
            cut_distance,
            phase,
            state,
        ),
    }
}

/// Turn only when a turn claims clearly more space than going straight.
fn choose_territory(options: &Options) -> Option<Direction> {
    let best_turn = options
        .turns
        .iter()
        .filter(|option| options.is_safe(option))
        .max_by(|a, b| a.territory.total_cmp(&b.territory))?;

    (best_turn.territory > options.straight.territory * NPC_TERRITORY_TURN_GAIN)
        .then_some(best_turn.direction)
}

/// Steer for the point a lightcycle will reach by the time the NPC gets
/// there, so that the NPC ends up across its path.
fn choose_intercept(
    context: &Context,
    target_id: Option<PlayerId>,
    state: &mut NpcState,
) -> Option<Direction> {
    let lightcycle = context.lightcycle;
    let alive = |id: &PlayerId| {
        context
            .arena
            .lightcycles
            .get(id)
            .is_some_and(|lightcycle| !lightcycle.dead)
    };

    let Some(target_id) = target_id
        .filter(alive)
        .or_else(|| find_intercept_target(context))
    else {
        state.cruise();
        return None;
    };
    state.behaviour = NpcBehaviour::Intercept {
        target_id: Some(target_id),
    };

    let target = &context.arena.lightcycles[&target_id];
    let distance = (target.position - lightcycle.position).length();
    let lead = (distance * target.speed / lightcycle.speed).min(150.0);
    let aim = target.position + target.direction.as_velocity() * lead;
    let aim = ArenaPoint::new(
        aim.x.clamp(0.0, context.arena.width),
        aim.y.clamp(0.0, context.arena.height),
    );

    let delta = aim - lightcycle.position;
    let forward = lightcycle.direction.as_velocity().dot(delta);
    let [a, b] = lightcycle.direction.perpendicular_directions();
    let (toward, lateral) = if a.as_velocity().dot(delta) > b.as_velocity().dot(delta) {
        (a, a.as_velocity().dot(delta))
    } else {
        (b, b.as_velocity().dot(delta))
    };

    if lateral < NPC_DRAFT_OFFSET {
        // Lined up with the aim point. If it is behind, turn around by way
        // of whichever side has more room.
        if forward >= 0.0 {
            return None;
        }
        let [a, b] = context.options.turns;
        return Some(if a.territory >= b.territory {
            a.direction
        } else {
            b.direction
        });
    }
    if forward > lateral * 1.5 {
        return None;
    }
    Some(toward)
}

/// The nearest other lightcycle, favouring human players.
fn find_intercept_target(context: &Context) -> Option<PlayerId> {
    context
        .arena
        .lightcycles
        .iter()
        .filter(|(id, lightcycle)| **id != context.npc_id && !lightcycle.dead)
        .map(|(id, lightcycle)| {
            let distance = (lightcycle.position - context.lightcycle.position).length();
            let weight = if context.npc_ids.contains(id) {
                1.0
            } else {
                0.7
            };
            (*id, distance * weight)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// Finds a trail, its own included, running the same way as the NPC, a
/// little to one side and stretching out ahead, that it could slipstream.
/// Returns the direction to move sideways in and how far to go.
fn find_draft_lane(context: &Context) -> Option<(Direction, f64)> {
    let lightcycle = context.lightcycle;
    let position = lightcycle.position;
    let velocity = lightcycle.direction.as_velocity();
    let is_vertical = matches!(lightcycle.direction, Direction::Up | Direction::Down);

    let mut best: Option<(Direction, f64)> = None;
    for lightribbon in context.arena.lightribbons.values() {
        for (from, to) in lightribbon.segments() {
            let (lateral, parallel) = if is_vertical {
                (from.x - position.x, (from.x - to.x).abs() <= f64::EPSILON)
            } else {
                (from.y - position.y, (from.y - to.y).abs() <= f64::EPSILON)
            };
            if !parallel || lateral.abs() <= NPC_DRAFT_OFFSET + 2.0 || lateral.abs() > 70.0 {
                continue;
            }

            // How far ahead of the NPC the segment starts and ends.
            let (start, end) = {
                let a = velocity.dot(from - position);
                let b = velocity.dot(to - position);
                (a.min(b), a.max(b))
            };
            if end < 60.0 || start > 30.0 {
                continue;
            }

            let distance = lateral.abs() - NPC_DRAFT_OFFSET;
            if best.is_none_or(|(_, best)| distance < best) {
                let [a, b] = lightcycle.direction.perpendicular_directions();
                let toward = if a.as_velocity().dot(from - position) > 0.0 {
                    a
                } else {
                    b
                };
                best = Some((toward, distance));
            }
        }
    }

    best
}

/// Build a notch/bump trap: turn perpendicular, travel a short distance, turn back.
/// This creates a pocket in the trail that other players can collide with.
fn choose_trap(
    context: &Context,
    step: u8,
    distance_in_step: f64,
    state: &mut NpcState,
) -> Option<Direction> {
    let lightcycle = context.lightcycle;
    let distance_in_step = distance_in_step + lightcycle.speed * context.delta_time;

    if distance_in_step < NPC_TRAP_SEGMENT_LENGTH {
        state.behaviour = NpcBehaviour::BuildTrap {
            step,
            distance_in_step,
        };
        return None;
    }

    let [a, b] = context.options.turns;
    match step {
        0 => {
            // First turn: either side.
            state.behaviour = NpcBehaviour::BuildTrap {
                step: 1,
                distance_in_step: 0.0,
            };
            Some(random_perpendicular(lightcycle.direction))
        }
        _ => {
            // Second turn: complete the notch towards the roomier side.
            state.cruise();
            Some(if a.ray >= b.ray {
                a.direction
            } else {
                b.direction
            })
        }
    }
}

/// Attempt to cut off a slipstreamer by turning perpendicular in front of them,
/// crossing their lane, then turning back. When the timing is bad the NPC ends
/// up behind the slipstreamer and effectively follows them.
fn choose_cutoff(
    context: &Context,
    target_id: PlayerId,
    cut_direction: Direction,
    original_direction: Direction,
    cut_distance: f64,
    phase: CutOffPhase,
    state: &mut NpcState,
) -> Option<Direction> {
    let lightcycle = context.lightcycle;

    match phase {
        CutOffPhase::Waiting(remaining) => {
            let remaining = remaining - context.delta_time;
            let phase = if remaining <= 0.0 {
                CutOffPhase::Crossing(0.0)
            } else {
                CutOffPhase::Waiting(remaining)
            };
            state.behaviour = NpcBehaviour::CutOff {
                target_id,
                cut_direction,
                original_direction,
                cut_distance,
                phase,
            };
            (remaining <= 0.0).then_some(cut_direction)
        }
        CutOffPhase::Crossing(distance) => {
            let distance = distance + lightcycle.speed * context.delta_time;
            if distance < cut_distance {
                state.behaviour = NpcBehaviour::CutOff {
                    target_id,
                    cut_direction,
                    original_direction,
                    cut_distance,
                    phase: CutOffPhase::Crossing(distance),
                };
                return None;
            }

            // Done crossing: turn back to the original direction.
            state.cruise();
            state.cutoff_cooldown = NPC_CUTOFF_COOLDOWN + 2.0;
            Some(original_direction)
        }
    }
}

/// Slipstream behind a target, overtake via speed boost, then cut across their path.
fn choose_slipstream_attack(
    context: &Context,
    target_id: PlayerId,
    target_direction: Direction,
    approach_direction: Direction,
    cut_distance: f64,
    phase: SlipstreamAttackPhase,
    state: &mut NpcState,
) -> Option<Direction> {
    let lightcycle = context.lightcycle;
    let target = context
        .arena
        .lightcycles
        .get(&target_id)
        .filter(|target| !target.dead);
    let on_course = target.filter(|target| target.direction == target_direction);
    let set_phase = |state: &mut NpcState, phase| {
        state.behaviour = NpcBehaviour::SlipstreamAttack {
            target_id,
            target_direction,
            approach_direction,
            cut_distance,
            phase,
        };
    };

    match phase {
        SlipstreamAttackPhase::Approaching => {
            let Some(target) = on_course else {
                state.abort();
                return None;
            };

            if lightcycle.direction != approach_direction {
                return Some(approach_direction);
            }

            let perp_dist = match target_direction {
                Direction::Up | Direction::Down => {
                    (target.position.x - lightcycle.position.x).abs()
                }
                Direction::Left | Direction::Right => {
                    (target.position.y - lightcycle.position.y).abs()
                }
            };
            if perp_dist > SLIPSTREAM_DISTANCE * 0.8 {
                return None;
            }

            // Within slipstream range: fall in behind the target.
            set_phase(state, SlipstreamAttackPhase::Drafting);
            Some(target_direction)
        }

        SlipstreamAttackPhase::Drafting => {
            let Some(target) = on_course else {
                state.abort();
                return None;
            };

            let ahead = target_direction
                .as_velocity()
                .dot(lightcycle.position - target.position);
            if ahead > 0.0 {
                set_phase(state, SlipstreamAttackPhase::Overtaking);
            }
            None
        }

        SlipstreamAttackPhase::Overtaking => {
            let ahead_of_target = match target {
                Some(target) => target_direction
                    .as_velocity()
                    .dot(lightcycle.position - target.position),
                // The target is gone, so just cut across.
                None => NPC_OVERTAKE_BUFFER,
            };
            if ahead_of_target < NPC_OVERTAKE_BUFFER {
                return None;
            }

            set_phase(state, SlipstreamAttackPhase::Cutting(0.0));
            Some(approach_direction)
        }

        SlipstreamAttackPhase::Cutting(distance) => {
            let distance = distance + lightcycle.speed * context.delta_time;
            if distance < cut_distance {
                set_phase(state, SlipstreamAttackPhase::Cutting(distance));
                return None;
            }

            // Done cutting: turn back to the target's direction.
            state.cruise();
            state.attack_cooldown = 5.0;
            Some(target_direction)
        }
    }
}

/// Find a nearby bike that the NPC can slipstream behind.
/// Returns `(target_id, target_direction, approach_direction, cut_distance)`.
fn find_slipstream_attack_target(
    arena: &Arena,
    npc_id: PlayerId,
    lightcycle: &Lightcycle,
) -> Option<(PlayerId, Direction, Direction, f64)> {
    let npc_pos = lightcycle.position;
    let npc_dir = lightcycle.direction;
    let npc_vel = npc_dir.as_velocity();

    let mut best: Option<(PlayerId, Direction, Direction, f64, f64)> = None;

    for (other_id, other_lc) in arena.lightcycles.iter() {
        if *other_id == npc_id || other_lc.dead {
            continue;
        }

        let target_dir = other_lc.direction;
        let target_pos = other_lc.position;
        let target_vel = target_dir.as_velocity();

        // NPC must be behind the target along the target's travel axis.
        let delta = target_pos - npc_pos;
        let ahead_dist = target_vel.x * delta.x + target_vel.y * delta.y;
        if ahead_dist < 20.0 || ahead_dist > 150.0 {
            continue;
        }

        // Perpendicular distance to target's current lane.
        let perp_dist = match target_dir {
            Direction::Up | Direction::Down => (target_pos.x - npc_pos.x).abs(),
            Direction::Left | Direction::Right => (target_pos.y - npc_pos.y).abs(),
        };

        // Skip if already within slipstream distance or way too far.
        if perp_dist <= SLIPSTREAM_DISTANCE || perp_dist > SLIPSTREAM_DISTANCE * 4.0 {
            continue;
        }

        let total_dist_sq = delta.x * delta.x + delta.y * delta.y;
        let cut_distance = SLIPSTREAM_DISTANCE;

        if npc_dir == target_dir {
            // Same direction: approach laterally then draft.
            let perp = npc_dir.perpendicular_directions();
            let s0 = perp[0].as_velocity().x * delta.x + perp[0].as_velocity().y * delta.y;
            let s1 = perp[1].as_velocity().x * delta.x + perp[1].as_velocity().y * delta.y;
            let approach_dir = if s0 > s1 { perp[0] } else { perp[1] };

            // Safety: enough room to approach.
            let approach_ahead = look_ahead_distance(arena, npc_pos, approach_dir);
            if approach_ahead < perp_dist {
                continue;
            }

            if best.is_none() || total_dist_sq < best.as_ref().unwrap().4 {
                best = Some((
                    *other_id,
                    target_dir,
                    approach_dir,
                    cut_distance,
                    total_dist_sq,
                ));
            }
        } else if !npc_dir.is_opposite(target_dir) {
            // Perpendicular: NPC is heading toward (or away from) the target's lane.
            let heading_toward = match target_dir {
                Direction::Up | Direction::Down => {
                    let dist = target_pos.x - npc_pos.x;
                    (npc_vel.x > 0.0 && dist > 0.0) || (npc_vel.x < 0.0 && dist < 0.0)
                }
                Direction::Left | Direction::Right => {
                    let dist = target_pos.y - npc_pos.y;
                    (npc_vel.y > 0.0 && dist > 0.0) || (npc_vel.y < 0.0 && dist < 0.0)
                }
            };

            if heading_toward {
                if best.is_none() || total_dist_sq < best.as_ref().unwrap().4 {
                    best = Some((*other_id, target_dir, npc_dir, cut_distance, total_dist_sq));
                }
            }
        }
    }

    best.map(|(id, tdir, adir, cdist, _)| (id, tdir, adir, cdist))
}

/// Find a bike that is slipstreaming this NPC's trail and is behind the NPC.
/// Returns `(target_id, target_position, target_speed)`.
fn find_slipstreamer_on_trail(
    arena: &Arena,
    npc_id: PlayerId,
    lightcycle: &Lightcycle,
) -> Option<(PlayerId, ArenaPoint, f64)> {
    let npc_ribbon = arena.lightribbons.get(&npc_id)?;
    let npc_vel = lightcycle.direction.as_velocity();

    for (other_id, other_lc) in arena.lightcycles.iter() {
        if *other_id == npc_id || other_lc.dead {
            continue;
        }

        // Only consider bikes going in the same direction (actually catching up).
        if other_lc.direction != lightcycle.direction {
            continue;
        }

        let other_is_vertical = matches!(other_lc.direction, Direction::Up | Direction::Down);
        let mut is_slipstreaming_us = false;

        for (from, to) in npc_ribbon.segments() {
            if segment_is_parallel_slipstream(other_lc.position, other_is_vertical, from, to) {
                is_slipstreaming_us = true;
                break;
            }
        }

        if !is_slipstreaming_us {
            if let Some(last_point) = npc_ribbon.last_point() {
                if segment_is_parallel_slipstream(
                    other_lc.position,
                    other_is_vertical,
                    last_point,
                    lightcycle.position,
                ) {
                    is_slipstreaming_us = true;
                }
            }
        }

        if is_slipstreaming_us {
            let delta = lightcycle.position - other_lc.position;
            let distance_behind = npc_vel.x * delta.x + npc_vel.y * delta.y;

            if distance_behind > 0.0 {
                return Some((*other_id, other_lc.position, other_lc.speed));
            }
        }
    }

    None
}

/// Cast a ray from `position` in `direction` and return distance to the first obstacle.
fn look_ahead_distance(arena: &Arena, position: ArenaPoint, direction: Direction) -> f64 {
    let vel = direction.as_velocity();
    let max_dist = (arena.width * arena.width + arena.height * arena.height).sqrt();
    let far_point = position + vel * max_dist;

    let ray = ArenaLine {
        from: position.to_untyped(),
        to: far_point.to_untyped(),
    };

    let mut min_dist = wall_distance(arena, position, direction);

    for (ribbon_id, lightribbon) in arena.lightribbons.iter() {
        for (from, to) in lightribbon.segments() {
            let segment = ArenaLine {
                from: from.to_untyped(),
                to: to.to_untyped(),
            };

            if let Some(intersection) = ray.intersection(&segment) {
                let d = ((intersection.x - position.x as f64).powi(2)
                    + (intersection.y - position.y as f64).powi(2))
                .sqrt();
                if d > 0.5 && d < min_dist {
                    min_dist = d;
                }
            }
        }

        // Check the live segment (ribbon tip → bike position) for other bikes.
        if let Some(other_cycle) = arena.lightcycles.get(ribbon_id) {
            if !other_cycle.dead {
                if let Some(last_point) = lightribbon.last_point() {
                    let live_segment = ArenaLine {
                        from: last_point.to_untyped(),
                        to: other_cycle.position.to_untyped(),
                    };
                    if let Some(intersection) = ray.intersection(&live_segment) {
                        let d = ((intersection.x - position.x as f64).powi(2)
                            + (intersection.y - position.y as f64).powi(2))
                        .sqrt();
                        if d > 0.5 && d < min_dist {
                            min_dist = d;
                        }
                    }
                }
            }
        }
    }

    // Check other alive bikes: both their projected paths (perpendicular
    // convergence) and whether they sit directly in our lane (head-on).
    for (_, other_cycle) in arena.lightcycles.iter() {
        if other_cycle.dead || other_cycle.position == position {
            continue;
        }

        // Direct in-lane check: is this bike ahead of us on the same axis?
        // This catches head-on collinear cases where intersection() returns
        // None because the segments overlap.
        let delta = other_cycle.position - position;
        let along = vel.x * delta.x + vel.y * delta.y; // dot product
        if along > 0.5 {
            // The other bike is ahead of us. Check perpendicular distance.
            let perp_dist = (vel.x * delta.y - vel.y * delta.x).abs();
            if perp_dist < 1.0 {
                // Directly in our lane.
                if along < min_dist {
                    min_dist = along;
                }
            }
        }

        // Projected path check for perpendicular convergence.
        let other_vel = other_cycle.direction.as_velocity();
        let projected_end = other_cycle.position + other_vel * NPC_LOOK_AHEAD_DISTANCE;
        let projected_segment = ArenaLine {
            from: other_cycle.position.to_untyped(),
            to: projected_end.to_untyped(),
        };
        if let Some(intersection) = ray.intersection(&projected_segment) {
            let d = ((intersection.x - position.x as f64).powi(2)
                + (intersection.y - position.y as f64).powi(2))
            .sqrt();
            if d > 0.5 && d < min_dist {
                min_dist = d;
            }
        }
    }

    min_dist
}

fn wall_distance(arena: &Arena, position: ArenaPoint, direction: Direction) -> f64 {
    match direction {
        Direction::Up => arena.height - position.y,
        Direction::Down => position.y,
        Direction::Right => arena.width - position.x,
        Direction::Left => position.x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_have_between_two_and_eight_lightcycles() {
        for humans in 1..=ARENA_MAX_PLAYERS {
            let totals: Vec<usize> = (0..500)
                .map(|_| humans + npc_count_for_arena(humans))
                .collect();
            let lowest = humans.max(MIN_LIGHTCYCLES);
            assert!(
                totals
                    .iter()
                    .all(|total| (lowest..=ARENA_MAX_PLAYERS).contains(total))
            );
            // Every possible total turns up.
            for total in lowest..=ARENA_MAX_PLAYERS {
                assert!(
                    totals.contains(&total),
                    "{humans} humans never made {total}"
                );
            }
        }
    }
}
