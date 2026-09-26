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
/// How long after turning an NPC keeps going straight, unless its way ahead
/// is blocked or its behaviour asks to turn. Without it, an NPC next to a
/// faster lightcycle can turn back and forth every update, as the space
/// ahead of it changes hands from one update to the next.
const NPC_SETTLE_TIME: f64 = 0.25;
const NPC_ATTACK_COOLDOWN: f64 = 2.0;
const NPC_SIDESTEP_COOLDOWN: f64 = 2.0;
/// How far an NPC must have come in a straight line before doubling back
/// alongside its own trail is worth it.
const NPC_HAIRPIN_MIN_RUN: f64 = 60.0;
/// How far to the side of its target's lane a shadowing NPC rides: outside
/// slipstream range, so neither gets a boost from the other's trail.
const NPC_SHADOW_OFFSET: f64 = 22.0;
/// How far to the side of a trail an NPC rides when drafting it. More than a
/// planning grid cell, so the trail doesn't block the NPC's own cells.
const NPC_DRAFT_OFFSET: f64 = 7.0;
/// A Territory NPC only turns when a turn gains this much over going straight.
const NPC_TERRITORY_TURN_GAIN: f64 = 1.05;

/// Phase within the CutOff behaviour.
#[derive(Debug, Clone, Copy)]
pub enum CutOffPhase {
    /// Riding on until the slipstreamer is close enough that the NPC only
    /// just gets across its lane first. `lead` is how many seconds before
    /// the slipstreamer the NPC aims to cross.
    Waiting { lead: f64 },
    /// Crossing the slipstreamer's lane, `distance` in all.
    Crossing { travelled: f64, distance: f64 },
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
    /// Rides alongside another lightcycle, turning when it turns, and waits
    /// for it to slip up.
    Shadow,
    /// Lays long straight walls across the arena to box others in.
    Boxer,
    /// Chases other lightcycles and takes every opening to cut them off,
    /// accepting moves that leave it much less room.
    Kamikaze,
}

impl Style {
    const ALL: [Style; 7] = [
        Style::Hunter,
        Style::Racer,
        Style::Trickster,
        Style::Sentinel,
        Style::Shadow,
        Style::Boxer,
        Style::Kamikaze,
    ];

    /// How likely each plan is to be picked when a behaviour runs out.
    fn plans(self) -> &'static [(Plan, u32)] {
        match self {
            Style::Hunter => &[
                (Plan::Territory, 45),
                (Plan::Intercept, 30),
                (Plan::Cruise, 15),
                (Plan::Draft, 10),
                (Plan::Hairpin, 5),
            ],
            Style::Racer => &[
                (Plan::Draft, 35),
                (Plan::Cruise, 20),
                (Plan::Intercept, 20),
                (Plan::Hairpin, 15),
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
                (Plan::Hairpin, 5),
            ],
            Style::Sentinel => &[
                (Plan::Cruise, 40),
                (Plan::Territory, 40),
                (Plan::Draft, 15),
                (Plan::BuildTrap, 5),
            ],
            Style::Shadow => &[
                (Plan::Shadow, 60),
                (Plan::Draft, 15),
                (Plan::Cruise, 15),
                (Plan::Territory, 10),
            ],
            Style::Boxer => &[
                (Plan::Divide, 55),
                (Plan::Territory, 35),
                (Plan::Cruise, 10),
            ],
            Style::Kamikaze => &[
                (Plan::Intercept, 55),
                (Plan::Shadow, 20),
                (Plan::Draft, 15),
                (Plan::Hairpin, 10),
            ],
        }
    }
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
    /// Chance, in percent, of sidestepping a faster lightcycle drawing level
    /// alongside, before it can cut in.
    pub anticipation: u32,
    /// Chance, in percent, of holding a grudge against a lightcycle that
    /// slipstreams it or tries to cut in on it.
    pub temper: u32,
    /// This NPC's own weighting of its style's plans.
    plans: Vec<(Plan, u32)>,
}

impl Personality {
    /// A personality of the given style. Each NPC's numbers vary a little
    /// around its style's, so no two NPCs of a style play quite alike.
    pub fn new(style: Style) -> Self {
        #[rustfmt::skip]
        let (clearance, tolerance, cutoff_chance, cutoff_accuracy, attack_chance, anticipation, temper) =
            match style {
                Style::Hunter => (random_between(8.0, 14.0), 0.10, 85, 75, 60, 60, 50),
                Style::Racer => (random_between(6.0, 12.0), 0.15, 70, 65, 90, 70, 30),
                Style::Trickster => (random_between(10.0, 25.0), 0.25, 60, 55, 40, 50, 40),
                Style::Sentinel => (random_between(20.0, 35.0), 0.05, 90, 80, 20, 80, 20),
                Style::Shadow => (random_between(10.0, 16.0), 0.12, 80, 85, 75, 90, 60),
                Style::Boxer => (random_between(3.0, 6.0), 0.08, 50, 60, 20, 40, 20),
                Style::Kamikaze => (random_between(4.0, 8.0), 0.40, 100, 50, 100, 20, 90),
            };
        let vary = |percent: u32| (percent as f64 * random_between(0.8, 1.2)).min(100.0) as u32;

        Self {
            style,
            clearance,
            tolerance: tolerance * random_between(0.7, 1.3),
            cutoff_chance: vary(cutoff_chance),
            cutoff_accuracy: vary(cutoff_accuracy),
            attack_chance: vary(attack_chance),
            anticipation: vary(anticipation),
            temper: vary(temper),
            plans: style
                .plans()
                .iter()
                .map(|(plan, weight)| (*plan, vary(*weight).max(1)))
                .collect(),
        }
    }

    fn random() -> Self {
        Self::new(Style::ALL[random_below(Style::ALL.len() as u32) as usize])
    }

    /// Picks the next behaviour at random, weighted by this NPC's plans. An
    /// NPC with a grudge often goes after whoever it holds it against.
    fn pick_behaviour(&self, lightcycle: &Lightcycle, grudge: Option<PlayerId>) -> NpcBehaviour {
        if let Some(target_id) = grudge
            && percent_chance(60)
        {
            return if percent_chance(50) {
                NpcBehaviour::Shadow {
                    target_id: Some(target_id),
                }
            } else {
                NpcBehaviour::Intercept {
                    target_id: Some(target_id),
                }
            };
        }

        let mut roll = random_below(self.plans.iter().map(|(_, weight)| weight).sum());
        let plan = self
            .plans
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
    Hairpin,
    Shadow,
    Divide,
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
            Plan::Hairpin => NpcBehaviour::Hairpin,
            Plan::Shadow => NpcBehaviour::Shadow { target_id: None },
            Plan::Divide => NpcBehaviour::Divide,
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
    /// Double back alongside the NPC's own trail to slipstream it.
    Hairpin,
    /// Ride level with a lightcycle a little to one side of it, turning when
    /// it turns.
    Shadow { target_id: Option<PlayerId> },
    /// Turn onto the longest open line and ride it out, walling off the
    /// arena.
    Divide,
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
    /// Cut off a detected slipstreamer by turning in front of them, going
    /// `overshoot` past their lane before turning back.
    CutOff {
        target_id: PlayerId,
        cut_direction: Direction,
        original_direction: Direction,
        overshoot: f64,
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

    /// The lightcycle this behaviour is attacking, whose path ahead the NPC
    /// means to cross.
    fn target(&self) -> Option<PlayerId> {
        match self {
            NpcBehaviour::CutOff { target_id, .. }
            | NpcBehaviour::SlipstreamAttack { target_id, .. } => Some(*target_id),
            _ => None,
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
    /// Cooldown before the NPC will react to a new lightcycle drawing level.
    pub sidestep_cooldown: f64,
    /// A lightcycle this NPC holds a grudge against, and for how many more
    /// seconds.
    pub grudge: Option<(PlayerId, f64)>,
    /// Seconds since the NPC last turned.
    pub since_turn: f64,
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
            sidestep_cooldown: 0.0,
            grudge: None,
            since_turn: 0.0,
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

    /// Maybe start holding a grudge against `against`, depending on temper.
    fn provoke(&mut self, against: PlayerId) {
        if self.grudge.is_none_or(|(id, _)| id != against)
            && percent_chance(self.personality.temper)
        {
            self.grudge = Some((against, random_between(8.0, 15.0)));
        }
    }

    /// The lightcycle this NPC holds a grudge against, if it is still alive.
    fn grudge_target(&self, arena: &Arena) -> Option<PlayerId> {
        self.grudge.map(|(id, _)| id).filter(|id| {
            arena
                .lightcycles
                .get(id)
                .is_some_and(|lightcycle| !lightcycle.dead)
        })
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
        NpcBehaviour::Draft | NpcBehaviour::Hairpin | NpcBehaviour::Divide => 1.0,
        NpcBehaviour::Shadow { .. } => random_between(5.0, 9.0),
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
            if incompatible_set.contains(&name) {
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
        state.grudge = state
            .grudge
            .map(|(id, remaining)| (id, remaining - delta_time))
            .filter(|(_, remaining)| *remaining > 0.0);
        state.time_remaining -= delta_time;
        if state.time_remaining <= 0.0 {
            state.abort();
            let grudge = state.grudge_target(arena);
            let behaviour = state.personality.pick_behaviour(lightcycle, grudge);
            state.set_behaviour(behaviour);
        }

        state.cutoff_cooldown = (state.cutoff_cooldown - delta_time).max(0.0);
        state.attack_cooldown = (state.attack_cooldown - delta_time).max(0.0);
        state.sidestep_cooldown = (state.sidestep_cooldown - delta_time).max(0.0);
        react_to_slipstreamer(arena, *npc_id, lightcycle, state);
        react_to_overtaker(arena, *npc_id, lightcycle, state);
        look_for_slipstream_attack(arena, *npc_id, lightcycle, state);

        let options = planner.options(*npc_id, lightcycle, delta_time, state.behaviour.target());
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
        let settling = request.is_none()
            && state.since_turn < NPC_SETTLE_TIME
            && options.is_safe(&options.straight);
        let direction = if settling {
            lightcycle.direction
        } else {
            if overruled && state.behaviour.is_maneuver() {
                state.abort();
            }
            direction
        };

        state.since_turn += delta_time;
        if direction != lightcycle.direction {
            state.since_turn = 0.0;
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
    // An NPC already attacking cuts across on its own terms, including when
    // its target falls in behind it.
    let attacking = matches!(
        state.behaviour,
        NpcBehaviour::CutOff { .. } | NpcBehaviour::SlipstreamAttack { .. }
    );
    if attacking || state.cutoff_cooldown > 0.0 {
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

    let distance_behind = lightcycle
        .direction
        .as_velocity()
        .dot(lightcycle.position - target_pos);
    if distance_behind <= 5.0
        || catch_up_time(distance_behind, lightcycle.speed, target_speed).is_none()
    {
        return;
    }

    state.provoke(target_id);
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

    let lead = if percent_chance(personality.cutoff_accuracy) {
        random_between(0.1, 0.3)
    } else {
        // Too early: the slipstreamer has time to see it coming and turn.
        random_between(0.7, 1.1)
    };

    state.set_behaviour(NpcBehaviour::CutOff {
        target_id,
        cut_direction,
        original_direction: lightcycle.direction,
        overshoot: random_between(10.0, 20.0),
        phase: CutOffPhase::Waiting { lead },
    });
}

/// When a faster lightcycle draws level alongside, sidestep away from it
/// before it can get past and cut in. If it cuts in anyway, it runs into the
/// NPC's new lane; if not, the NPC can fall in behind it and slipstream it.
fn react_to_overtaker(
    arena: &Arena,
    npc_id: PlayerId,
    lightcycle: &Lightcycle,
    state: &mut NpcState,
) {
    let committed = matches!(
        state.behaviour,
        NpcBehaviour::CutOff { .. }
            | NpcBehaviour::SlipstreamAttack { .. }
            | NpcBehaviour::LaneChange { .. }
    );
    if committed || state.sidestep_cooldown > 0.0 {
        return;
    }
    let Some((overtaker_id, away)) = find_overtaker(arena, npc_id, lightcycle) else {
        return;
    };

    state.sidestep_cooldown = NPC_SIDESTEP_COOLDOWN;
    state.provoke(overtaker_id);
    if !percent_chance(state.personality.anticipation) {
        return;
    }
    state.set_behaviour(NpcBehaviour::LaneChange {
        toward: away,
        resume: lightcycle.direction,
        remaining: random_between(18.0, 28.0),
    });
    state.attack_cooldown = 0.0;
}

/// Finds a faster lightcycle going the same way, close alongside and about
/// level. Returns it and the direction away from it.
fn find_overtaker(
    arena: &Arena,
    npc_id: PlayerId,
    lightcycle: &Lightcycle,
) -> Option<(PlayerId, Direction)> {
    let forward = lightcycle.direction.as_velocity();
    let [a, b] = lightcycle.direction.perpendicular_directions();

    arena
        .lightcycles
        .iter()
        .filter(|(id, other)| {
            **id != npc_id
                && !other.dead
                && other.direction == lightcycle.direction
                && other.speed > lightcycle.speed + 5.0
        })
        .find_map(|(id, other)| {
            let delta = other.position - lightcycle.position;
            let ahead = forward.dot(delta);
            let side = if a.as_velocity().dot(delta) > 0.0 {
                a
            } else {
                b
            };
            let lateral = side.as_velocity().dot(delta);
            ((-4.0..=10.0).contains(&ahead) && lateral <= SLIPSTREAM_DISTANCE)
                .then(|| (*id, opposite(side)))
        })
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
        Direction::Left => Direction::Right,
        Direction::Right => Direction::Left,
    }
}

/// Seconds until a slipstreamer `distance` behind a lightcycle moving at
/// `speed` draws level with it, or `None` if it never will. The slipstreamer
/// keeps gaining speed until it reaches `LIGHTCYCLE_BOOST_SPEED`.
fn catch_up_time(distance: f64, speed: f64, slipstreamer_speed: f64) -> Option<f64> {
    let acceleration = (LIGHTCYCLE_BOOST_SPEED - LIGHTCYCLE_SPEED) / SLIPSTREAM_RAMP_UP_SECONDS;
    let closing_speed = slipstreamer_speed - speed;
    let ramp_time = (LIGHTCYCLE_BOOST_SPEED - slipstreamer_speed).max(0.0) / acceleration;
    let ramp_distance = closing_speed * ramp_time + 0.5 * acceleration * ramp_time * ramp_time;

    if distance <= ramp_distance {
        // Solve distance = closing_speed * t + acceleration * t² / 2.
        let discriminant = closing_speed * closing_speed + 2.0 * acceleration * distance;
        return Some((discriminant.sqrt() - closing_speed) / acceleration);
    }

    let final_closing_speed = LIGHTCYCLE_BOOST_SPEED - speed;
    (final_closing_speed > 1.0)
        .then(|| ramp_time + (distance - ramp_distance) / final_closing_speed)
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

    let grudge = state.grudge_target(arena);
    let target = find_slipstream_attack_target(arena, npc_id, lightcycle, grudge);
    let chance = |target_id| {
        let chance = state.personality.attack_chance;
        if Some(target_id) == grudge {
            chance + 30
        } else {
            chance
        }
    };
    match target {
        Some((target_id, target_direction, approach_direction, cut_distance))
            if percent_chance(chance(target_id)) =>
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
        NpcBehaviour::Hairpin => {
            match start_hairpin(context) {
                Some(behaviour) => state.set_behaviour(behaviour),
                None => state.cruise(),
            }
            choose_direction(context, state)
        }
        NpcBehaviour::Shadow { target_id } => choose_shadow(context, target_id, state),
        NpcBehaviour::Divide => {
            let options = context.options;
            let longest = std::iter::once(&options.straight)
                .chain(options.turns.iter())
                .filter(|option| options.is_safe(option))
                .max_by(|a, b| a.ray.total_cmp(&b.ray))
                .map(|option| option.direction);
            state.cruise();
            state.time_remaining = random_between(4.0, 7.0);
            longest.filter(|direction| *direction != lightcycle.direction)
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
            overshoot,
            phase,
        } => choose_cutoff(
            context,
            target_id,
            cut_direction,
            original_direction,
            overshoot,
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
        .or_else(|| find_intercept_target(context, state.grudge_target(context.arena)))
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

/// `preferred` if it is alive, or else the nearest other lightcycle,
/// favouring human players.
fn find_intercept_target(context: &Context, preferred: Option<PlayerId>) -> Option<PlayerId> {
    if preferred.is_some() {
        return preferred;
    }
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

/// A turn onto a lane just beside the NPC's own trail, from which to double
/// back alongside it and slipstream it, when the NPC has come a long way in
/// a straight line and the lane back is clear.
fn start_hairpin(context: &Context) -> Option<NpcBehaviour> {
    let lightcycle = context.lightcycle;
    // The trail's last point follows the lightcycle, so the straight run
    // starts at the point before it.
    let path = context
        .arena
        .lightribbons
        .get(&context.npc_id)?
        .paths
        .last()?;
    let corner = path.get(path.len().checked_sub(2)?)?;
    let run = (lightcycle.position - *corner).length();
    if run < NPC_HAIRPIN_MIN_RUN {
        return None;
    }

    let back = opposite(lightcycle.direction);
    let offset = random_between(NPC_DRAFT_OFFSET, NPC_DRAFT_OFFSET + 4.0);
    context
        .options
        .turns
        .iter()
        .filter(|turn| turn.ray > offset + NPC_DRAFT_OFFSET)
        .filter(|turn| {
            let lane = lightcycle.position + turn.direction.as_velocity() * offset;
            look_ahead_distance(context.arena, lane, back, None) > run * 0.75
        })
        .max_by(|a, b| a.space.total_cmp(&b.space))
        .map(|turn| NpcBehaviour::LaneChange {
            toward: turn.direction,
            resume: back,
            remaining: offset,
        })
}

/// Ride level with a lightcycle, `NPC_SHADOW_OFFSET` to one side of its
/// lane, and turn when it turns. While well behind, ride close enough to its
/// trail to slipstream it and catch up.
fn choose_shadow(
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
        .or_else(|| find_intercept_target(context, state.grudge_target(context.arena)))
    else {
        state.cruise();
        return None;
    };
    state.behaviour = NpcBehaviour::Shadow {
        target_id: Some(target_id),
    };

    let target = &context.arena.lightcycles[&target_id];
    let to_target = target.position - lightcycle.position;
    let [a, b] = lightcycle.direction.perpendicular_directions();
    let toward = if a.as_velocity().dot(to_target) > 0.0 {
        a
    } else {
        b
    };

    if lightcycle.direction == target.direction {
        // Alongside: move out or in to the right distance from its lane.
        let lateral = toward.as_velocity().dot(to_target);
        let behind = target.direction.as_velocity().dot(to_target);
        let (near, far) = if behind > 25.0 {
            (NPC_DRAFT_OFFSET * 0.5, SLIPSTREAM_DISTANCE - 2.0)
        } else {
            (NPC_SHADOW_OFFSET * 0.5, NPC_SHADOW_OFFSET * 1.5)
        };
        if lateral > far {
            Some(toward)
        } else if lateral < near {
            Some(opposite(toward))
        } else {
            None
        }
    } else if lightcycle.direction.is_opposite(target.direction) {
        Some(toward)
    } else {
        // Heading across its lane: straighten up alongside it, on this side
        // if already close enough, otherwise once across.
        let to_lane = lightcycle.direction.as_velocity().dot(to_target);
        let turn = if to_lane > NPC_SHADOW_OFFSET {
            false
        } else if to_lane >= NPC_SHADOW_OFFSET * 0.5 {
            true
        } else {
            to_lane <= -NPC_SHADOW_OFFSET * 0.5
        };
        turn.then_some(target.direction)
    }
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

/// Cut off a slipstreamer by turning across in front of them at the last
/// moment the NPC can still get across their lane first, then turning back
/// once past it. Gives up if the slipstreamer turns off, moves away, or gets
/// too close to cross in front of.
fn choose_cutoff(
    context: &Context,
    target_id: PlayerId,
    cut_direction: Direction,
    original_direction: Direction,
    overshoot: f64,
    phase: CutOffPhase,
    state: &mut NpcState,
) -> Option<Direction> {
    let lightcycle = context.lightcycle;

    match phase {
        CutOffPhase::Waiting { lead } => {
            let target = context
                .arena
                .lightcycles
                .get(&target_id)
                .filter(|target| !target.dead && target.direction == original_direction)
                .filter(|_| lightcycle.direction == original_direction);
            let Some(target) = target else {
                state.abort();
                return None;
            };

            let behind = original_direction
                .as_velocity()
                .dot(lightcycle.position - target.position);
            let aside = cut_direction
                .as_velocity()
                .dot(target.position - lightcycle.position);
            // How long the slipstreamer takes to reach the line the NPC
            // would cross on, and how long the NPC takes to get past them.
            let target_time = catch_up_time(behind, 0.0, target.speed);
            let cross_time = (aside + 2.0) / lightcycle.speed;
            let in_reach = (0.0..=SLIPSTREAM_DISTANCE + 5.0).contains(&aside);
            match target_time {
                Some(target_time) if in_reach && target_time > cross_time + 0.05 => {
                    if target_time > cross_time + lead {
                        return None;
                    }
                    state.behaviour = NpcBehaviour::CutOff {
                        target_id,
                        cut_direction,
                        original_direction,
                        overshoot,
                        phase: CutOffPhase::Crossing {
                            travelled: 0.0,
                            distance: aside + overshoot,
                        },
                    };
                    Some(cut_direction)
                }
                _ => {
                    state.abort();
                    None
                }
            }
        }
        CutOffPhase::Crossing {
            travelled,
            distance,
        } => {
            let travelled = travelled + lightcycle.speed * context.delta_time;
            if travelled < distance {
                state.behaviour = NpcBehaviour::CutOff {
                    target_id,
                    cut_direction,
                    original_direction,
                    overshoot,
                    phase: CutOffPhase::Crossing {
                        travelled,
                        distance,
                    },
                };
                return None;
            }

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
            if perp_dist > NPC_DRAFT_OFFSET {
                return None;
            }

            // Close alongside the target's trail: fall in behind the target.
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
            // With the target gone, just cut across.
            if target.is_some() {
                let Some(target) = on_course else {
                    state.abort();
                    return None;
                };
                let ahead = target_direction
                    .as_velocity()
                    .dot(lightcycle.position - target.position);
                if ahead < 0.0 {
                    set_phase(state, SlipstreamAttackPhase::Drafting);
                    return None;
                }
                // Cut once the NPC can get across the target's lane before
                // the target reaches it.
                let aside = approach_direction
                    .as_velocity()
                    .dot(target.position - lightcycle.position)
                    .max(0.0);
                let cross_time = (aside + 2.0) / lightcycle.speed;
                if ahead / target.speed < cross_time + 0.05 {
                    return None;
                }
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
/// Favours `preferred`, if it is one of them.
fn find_slipstream_attack_target(
    arena: &Arena,
    npc_id: PlayerId,
    lightcycle: &Lightcycle,
    preferred: Option<PlayerId>,
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
        if !(20.0..=150.0).contains(&ahead_dist) {
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
        let total_dist_sq = if preferred == Some(*other_id) {
            total_dist_sq * 0.25
        } else {
            total_dist_sq
        };
        let cut_distance = SLIPSTREAM_DISTANCE;

        if npc_dir == target_dir {
            // Same direction: approach laterally then draft.
            let perp = npc_dir.perpendicular_directions();
            let s0 = perp[0].as_velocity().x * delta.x + perp[0].as_velocity().y * delta.y;
            let s1 = perp[1].as_velocity().x * delta.x + perp[1].as_velocity().y * delta.y;
            let approach_dir = if s0 > s1 { perp[0] } else { perp[1] };

            // Safety: enough room to approach.
            let approach_ahead = look_ahead_distance(arena, npc_pos, approach_dir, None);
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

            if heading_toward && (best.is_none() || total_dist_sq < best.as_ref().unwrap().4) {
                best = Some((*other_id, target_dir, npc_dir, cut_distance, total_dist_sq));
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

        if !is_slipstreaming_us
            && let Some(last_point) = npc_ribbon.last_point()
            && segment_is_parallel_slipstream(
                other_lc.position,
                other_is_vertical,
                last_point,
                lightcycle.position,
            )
        {
            is_slipstreaming_us = true;
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
/// The path ahead of the lightcycle `ignoring` doesn't count as an obstacle,
/// for an NPC that means to cross it before that lightcycle gets there.
fn look_ahead_distance(
    arena: &Arena,
    position: ArenaPoint,
    direction: Direction,
    ignoring: Option<PlayerId>,
) -> f64 {
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
                let d = ((intersection.x - position.x).powi(2)
                    + (intersection.y - position.y).powi(2))
                .sqrt();
                if d > 0.5 && d < min_dist {
                    min_dist = d;
                }
            }
        }

        // Check the live segment (ribbon tip → bike position) for other bikes.
        if let Some(other_cycle) = arena.lightcycles.get(ribbon_id)
            && !other_cycle.dead
            && let Some(last_point) = lightribbon.last_point()
        {
            let live_segment = ArenaLine {
                from: last_point.to_untyped(),
                to: other_cycle.position.to_untyped(),
            };
            if let Some(intersection) = ray.intersection(&live_segment) {
                let d = ((intersection.x - position.x).powi(2)
                    + (intersection.y - position.y).powi(2))
                .sqrt();
                if d > 0.5 && d < min_dist {
                    min_dist = d;
                }
            }
        }
    }

    // Check other alive bikes: both their projected paths (perpendicular
    // convergence) and whether they sit directly in our lane (head-on).
    for (other_id, other_cycle) in arena.lightcycles.iter() {
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
        if ignoring == Some(*other_id) {
            continue;
        }
        let other_vel = other_cycle.direction.as_velocity();
        let projected_end = other_cycle.position + other_vel * NPC_LOOK_AHEAD_DISTANCE;
        let projected_segment = ArenaLine {
            from: other_cycle.position.to_untyped(),
            to: projected_end.to_untyped(),
        };
        if let Some(intersection) = ray.intersection(&projected_segment) {
            let d = ((intersection.x - position.x).powi(2) + (intersection.y - position.y).powi(2))
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
    use crate::server::ArenaVector;

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

    #[test]
    fn slipstreamers_are_expected_to_speed_up() {
        // Just started slipstreaming, 20 units back: about 1.9 s at 11 units/s².
        let time = catch_up_time(20.0, LIGHTCYCLE_SPEED, LIGHTCYCLE_SPEED + 0.55).unwrap();
        assert!((1.7..2.0).contains(&time), "{time}");

        // Already at full boost.
        let time = catch_up_time(55.0, LIGHTCYCLE_SPEED, LIGHTCYCLE_BOOST_SPEED).unwrap();
        assert!((time - 1.0).abs() < 1e-9, "{time}");

        // Never catches a boosted lightcycle.
        assert!(catch_up_time(20.0, LIGHTCYCLE_BOOST_SPEED, LIGHTCYCLE_SPEED).is_none());
    }

    /// A running arena with an NPC and another lightcycle, both heading up
    /// with a stretch of trail behind them.
    fn duel(
        npc_start: ArenaPoint,
        other_start: ArenaPoint,
        state: NpcState,
    ) -> (Arena, PlayerId, PlayerId) {
        let mut arena = Arena {
            started: Some(Utc::now() - chrono::Duration::seconds(1)),
            ..Default::default()
        };
        let npc = PlayerId::default();
        let other = PlayerId::default();
        for (id, start) in [(npc, npc_start), (other, other_start)] {
            arena.lightcycles.insert(
                id,
                Lightcycle {
                    position: start,
                    ..Default::default()
                },
            );
            arena.lightribbons.insert(
                id,
                Lightribbon {
                    paths: vec![vec![start - ArenaVector::new(0.0, 30.0), start]],
                },
            );
        }
        arena.npc_states.insert(npc, state);
        (arena, npc, other)
    }

    /// Runs the arena for up to `seconds`, stopping at the first death.
    fn run_until_death(arena: &mut Arena, seconds: f64) {
        for _ in 0..(seconds / 0.05) as usize {
            arena.update(0.05);
            arena.clear_updates();
            if arena.lightcycles.values().any(|lightcycle| lightcycle.dead) {
                return;
            }
        }
    }

    #[test]
    fn npcs_cut_off_lightcycles_slipstreaming_them() {
        let mut state = NpcState {
            personality: Personality::new(Style::Sentinel),
            ..Default::default()
        };
        state.personality.cutoff_chance = 100;
        state.personality.cutoff_accuracy = 100;
        state.time_remaining = f64::INFINITY;
        let (mut arena, npc, other) = duel(
            ArenaPoint::new(280.0, 60.0),
            ArenaPoint::new(288.0, 30.0),
            state,
        );

        run_until_death(&mut arena, 4.0);
        assert_eq!(
            arena.lightcycles[&other].death_cause,
            Some(DeathCause::Trail(npc))
        );
    }

    #[test]
    fn npcs_slipstream_overtake_and_cut_in() {
        let mut state = NpcState {
            personality: Personality::new(Style::Racer),
            ..Default::default()
        };
        state.personality.attack_chance = 100;
        state.time_remaining = f64::INFINITY;
        let (mut arena, npc, other) = duel(
            ArenaPoint::new(250.0, 40.0),
            ArenaPoint::new(280.0, 90.0),
            state,
        );

        run_until_death(&mut arena, 9.0);
        assert_eq!(
            arena.lightcycles[&other].death_cause,
            Some(DeathCause::Trail(npc))
        );
    }

    #[test]
    fn npcs_double_back_alongside_their_own_trail_for_speed() {
        let mut state = NpcState {
            personality: Personality::new(Style::Racer),
            ..Default::default()
        };
        state.time_remaining = f64::INFINITY;
        // The other lightcycle is far off, out of the way.
        let (mut arena, npc, _) = duel(
            ArenaPoint::new(280.0, 40.0),
            ArenaPoint::new(20.0, 500.0),
            state,
        );
        run_until_death(&mut arena, 1.5);

        arena
            .npc_states
            .get_mut(&npc)
            .unwrap()
            .set_behaviour(NpcBehaviour::Hairpin);
        let mut doubled_back = false;
        let mut top_speed: f64 = 0.0;
        for _ in 0..60 {
            run_until_death(&mut arena, 0.05);
            let lightcycle = arena.lightcycles[&npc];
            doubled_back |= lightcycle.direction == Direction::Down;
            top_speed = top_speed.max(lightcycle.speed);
        }
        assert!(doubled_back);
        assert!(top_speed > LIGHTCYCLE_SPEED + 10.0, "{top_speed}");
        assert!(!arena.lightcycles[&npc].dead);
    }

    #[test]
    fn npcs_sidestep_a_faster_lightcycle_drawing_level() {
        let mut state = NpcState {
            personality: Personality::new(Style::Sentinel),
            ..Default::default()
        };
        state.personality.anticipation = 100;
        state.personality.cutoff_chance = 0;
        state.time_remaining = f64::INFINITY;
        let (mut arena, npc, other) = duel(
            ArenaPoint::new(280.0, 60.0),
            ArenaPoint::new(290.0, 58.0),
            state,
        );
        arena.lightcycles.get_mut(&other).unwrap().speed = LIGHTCYCLE_BOOST_SPEED;

        run_until_death(&mut arena, 0.05);
        assert!(matches!(
            arena.npc_states[&npc].behaviour,
            NpcBehaviour::LaneChange {
                toward: Direction::Left,
                resume: Direction::Up,
                ..
            }
        ));
    }
}
