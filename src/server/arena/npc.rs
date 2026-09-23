use super::*;
use rand_core::{OsRng, RngCore};

const NPC_COUNT: usize = 2;
const NPC_LOOK_AHEAD_DISTANCE: f64 = 40.0;
/// If an obstacle is closer than this, always do emergency survival regardless of behaviour.
const NPC_CRITICAL_DISTANCE: f64 = 15.0;
const NPC_TRAP_SEGMENT_LENGTH: f64 = 30.0;

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

/// NPC behaviour mode.
#[derive(Debug, Clone)]
pub enum NpcBehaviour {
    /// Standard obstacle avoidance — the original behaviour.
    Survive,
    /// Steer toward the nearest other lightcycle.
    Chase,
    /// Insert one random perpendicular turn, then revert to Survive.
    RandomTurn,
    /// Build a notch/bump trap: two turns that create a pocket in the trail.
    /// `step` tracks which segment we're on (0 = first leg, 1 = second leg).
    /// `distance_in_step` accumulates how far we've traveled in the current leg.
    BuildTrap { step: u8, distance_in_step: f64 },
}

#[derive(Debug, Clone)]
pub struct NpcState {
    pub behaviour: NpcBehaviour,
    /// Seconds remaining before switching to a new random behaviour.
    pub time_remaining: f64,
}

impl Default for NpcState {
    fn default() -> Self {
        Self {
            behaviour: NpcBehaviour::Survive,
            time_remaining: random_duration_for(&NpcBehaviour::Survive),
        }
    }
}

fn random_duration_for(behaviour: &NpcBehaviour) -> f64 {
    match behaviour {
        NpcBehaviour::Survive => 2.0 + (OsRng.next_u32() % 300) as f64 / 100.0,
        NpcBehaviour::Chase => 1.5 + (OsRng.next_u32() % 200) as f64 / 100.0,
        NpcBehaviour::RandomTurn => 0.5,
        NpcBehaviour::BuildTrap { .. } => 5.0,
    }
}

fn pick_random_behaviour() -> NpcBehaviour {
    let roll = OsRng.next_u32() % 100;
    match roll {
        0..=41 => NpcBehaviour::Survive,
        42..=61 => NpcBehaviour::Chase,
        62..=94 => NpcBehaviour::RandomTurn,
        _ => NpcBehaviour::BuildTrap {
            step: 0,
            distance_in_step: 0.0,
        },
    }
}

pub fn create_npc_players(count: usize) -> Vec<Player> {
    let mut pool_remaining = NPC_PLAYER_POOL.to_vec();
    let mut players = Vec::with_capacity(count);

    for _ in 0..count {
        if pool_remaining.is_empty() {
            break;
        }

        let index = OsRng.next_u32() as usize % pool_remaining.len();
        let (name, color) = pool_remaining.remove(index);

        // remove incompatible NPCs from the pool
        for incompatible_set in INCOMPATIBLE_NPC_SETS {
            if incompatible_set.iter().any(|n| *n == name) {
                for incompatible_name in *incompatible_set {
                    if let Some(pos) = pool_remaining.iter().position(|(n, _)| *n == *incompatible_name) {
                        pool_remaining.remove(pos);
                    }
                }
            }
        }

        players.push(Player {
            id: PlayerId::default(),
            name: name.to_string(),
            color,
        });
    }

    players
}

pub fn npc_count_for_arena(human_player_count: usize) -> usize {
    let total = human_player_count + NPC_COUNT;
    if total > ARENA_MAX_PLAYERS {
        ARENA_MAX_PLAYERS - human_player_count
    } else {
        NPC_COUNT
    }
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

    for (npc_id, state) in npc_states.iter_mut() {
        let lightcycle = match arena.lightcycles.get(npc_id) {
            Some(lc) if !lc.dead => lc,
            _ => continue,
        };

        // Tick the behaviour timer and potentially switch behaviours.
        state.time_remaining -= delta_time;
        if state.time_remaining <= 0.0 {
            state.behaviour = pick_random_behaviour();
            state.time_remaining = random_duration_for(&state.behaviour);
        }

        if let Some(direction) = choose_direction(arena, lightcycle, *npc_id, state, delta_time) {
            updates.push(ArenaUpdate::UpdateLightribbonAppendPoint(
                *npc_id,
                lightcycle.position,
            ));
            updates.push(ArenaUpdate::UpdateLightcycleDirection(*npc_id, direction));
        }
    }

    updates
}

fn choose_direction(
    arena: &Arena,
    lightcycle: &Lightcycle,
    npc_id: PlayerId,
    state: &mut NpcState,
    delta_time: f64,
) -> Option<Direction> {
    let forward_dist = look_ahead_distance(arena, lightcycle.position, lightcycle.direction);

    // Emergency survival override: if we're about to hit something very soon,
    // always do a survival turn regardless of the current behaviour.
    if forward_dist < NPC_CRITICAL_DISTANCE {
        return survival_turn(arena, lightcycle);
    }

    // Copy trap state out to avoid double-borrow of `state`.
    let trap_info = match &state.behaviour {
        NpcBehaviour::BuildTrap {
            step,
            distance_in_step,
        } => Some((*step, *distance_in_step)),
        _ => None,
    };

    match &state.behaviour {
        NpcBehaviour::Survive => choose_survive(arena, lightcycle, forward_dist),
        NpcBehaviour::Chase => choose_chase(arena, lightcycle, npc_id, forward_dist),
        NpcBehaviour::RandomTurn => {
            let dir = choose_random_turn(lightcycle);
            // Immediately revert to Survive after the random turn.
            state.behaviour = NpcBehaviour::Survive;
            state.time_remaining = random_duration_for(&NpcBehaviour::Survive);
            dir
        }
        NpcBehaviour::BuildTrap { .. } => {
            let (step, distance_in_step) = trap_info.unwrap();
            choose_trap(arena, lightcycle, step, distance_in_step, delta_time, forward_dist, state)
        }
    }
}

/// Original obstacle-avoidance behaviour.
fn choose_survive(
    arena: &Arena,
    lightcycle: &Lightcycle,
    forward_dist: f64,
) -> Option<Direction> {
    if forward_dist > NPC_LOOK_AHEAD_DISTANCE {
        return None;
    }
    survival_turn(arena, lightcycle)
}

/// Pick the perpendicular direction with more room.
fn survival_turn(arena: &Arena, lightcycle: &Lightcycle) -> Option<Direction> {
    let perp = lightcycle.direction.perpendicular_directions();
    let dist_a = look_ahead_distance(arena, lightcycle.position, perp[0]);
    let dist_b = look_ahead_distance(arena, lightcycle.position, perp[1]);

    let chosen = if dist_a > dist_b {
        perp[0]
    } else if dist_b > dist_a {
        perp[1]
    } else {
        perp[OsRng.next_u32() as usize % 2]
    };

    Some(chosen)
}

/// Steer toward the nearest other lightcycle.
fn choose_chase(
    arena: &Arena,
    lightcycle: &Lightcycle,
    npc_id: PlayerId,
    forward_dist: f64,
) -> Option<Direction> {
    // If we need to dodge soon, do a survival turn instead of chasing.
    if forward_dist < NPC_LOOK_AHEAD_DISTANCE {
        return survival_turn(arena, lightcycle);
    }

    // Find the nearest alive lightcycle that isn't us.
    let target_pos = arena
        .lightcycles
        .iter()
        .filter(|(id, lc)| **id != npc_id && !lc.dead)
        .map(|(_, lc)| {
            let dx = lc.position.x - lightcycle.position.x;
            let dy = lc.position.y - lightcycle.position.y;
            (dx * dx + dy * dy, lc.position)
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, pos)| pos);

    let target = match target_pos {
        Some(p) => p,
        None => return None, // nobody to chase
    };

    // Determine which perpendicular direction moves us closer to the target.
    let perp = lightcycle.direction.perpendicular_directions();
    let dx = target.x - lightcycle.position.x;
    let dy = target.y - lightcycle.position.y;

    // Pick the perpendicular direction whose velocity vector aligns with the delta.
    let score = |dir: Direction| -> f64 {
        let v = dir.as_velocity();
        v.x * dx + v.y * dy // dot product
    };

    let s0 = score(perp[0]);
    let s1 = score(perp[1]);

    // Only turn if we're not already heading roughly toward the target along our current axis.
    // Check if the target is mostly ahead of us.
    let forward_score = {
        let v = lightcycle.direction.as_velocity();
        v.x * dx + v.y * dy
    };
    let lateral_score = s0.abs().max(s1.abs());

    // If target is mostly ahead, keep going straight.
    if forward_score > 0.0 && forward_score > lateral_score * 1.5 {
        return None;
    }

    let chosen = if s0 > s1 { perp[0] } else { perp[1] };

    // Safety check: don't turn into something close.
    let chosen_dist = look_ahead_distance(arena, lightcycle.position, chosen);
    if chosen_dist < NPC_CRITICAL_DISTANCE {
        return None; // too dangerous, keep going straight
    }

    Some(chosen)
}

/// Insert a random perpendicular turn.
fn choose_random_turn(lightcycle: &Lightcycle) -> Option<Direction> {
    let perp = lightcycle.direction.perpendicular_directions();
    Some(perp[OsRng.next_u32() as usize % 2])
}

/// Build a notch/bump trap: turn perpendicular, travel a short distance, turn back.
/// This creates a pocket in the trail that other players can collide with.
fn choose_trap(
    arena: &Arena,
    lightcycle: &Lightcycle,
    step: u8,
    distance_in_step: f64,
    delta_time: f64,
    forward_dist: f64,
    state: &mut NpcState,
) -> Option<Direction> {
    // If we'd hit something soon, abort the trap and go to Survive.
    if forward_dist < NPC_LOOK_AHEAD_DISTANCE {
        state.behaviour = NpcBehaviour::Survive;
        state.time_remaining = random_duration_for(&NpcBehaviour::Survive);
        return survival_turn(arena, lightcycle);
    }

    let distance_in_step = distance_in_step + lightcycle.speed * delta_time;

    if distance_in_step < NPC_TRAP_SEGMENT_LENGTH {
        // Update the accumulated distance but don't turn yet.
        state.behaviour = NpcBehaviour::BuildTrap {
            step,
            distance_in_step,
        };
        return None;
    }

    // Time to make the next turn.
    let perp = lightcycle.direction.perpendicular_directions();

    match step {
        0 => {
            // First turn: pick a random perpendicular direction.
            let idx = OsRng.next_u32() as usize % 2;
            let chosen = perp[idx];
            let alt = perp[1 - idx];
            let chosen = if look_ahead_distance(arena, lightcycle.position, chosen)
                > NPC_CRITICAL_DISTANCE
            {
                chosen
            } else if look_ahead_distance(arena, lightcycle.position, alt) > NPC_CRITICAL_DISTANCE {
                alt
            } else {
                // Both directions blocked, abort trap.
                state.behaviour = NpcBehaviour::Survive;
                state.time_remaining = random_duration_for(&NpcBehaviour::Survive);
                return survival_turn(arena, lightcycle);
            };

            state.behaviour = NpcBehaviour::BuildTrap {
                step: 1,
                distance_in_step: 0.0,
            };
            Some(chosen)
        }
        1 => {
            // Second turn: complete the notch. Pick the direction with more room.
            let dist_a = look_ahead_distance(arena, lightcycle.position, perp[0]);
            let dist_b = look_ahead_distance(arena, lightcycle.position, perp[1]);
            let chosen = if dist_a >= dist_b { perp[0] } else { perp[1] };

            // Trap complete, switch back to Survive.
            state.behaviour = NpcBehaviour::Survive;
            state.time_remaining = random_duration_for(&NpcBehaviour::Survive);
            Some(chosen)
        }
        _ => {
            state.behaviour = NpcBehaviour::Survive;
            state.time_remaining = random_duration_for(&NpcBehaviour::Survive);
            None
        }
    }
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
        for line in lightribbon.points.windows(2) {
            let segment = ArenaLine {
                from: line[0].to_untyped(),
                to: line[1].to_untyped(),
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
                if let Some(last_point) = lightribbon.points.last() {
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
        let projected_end =
            other_cycle.position + other_vel * NPC_LOOK_AHEAD_DISTANCE;
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
