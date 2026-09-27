use super::npc;
use super::*;

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum ArenaInput {
    Start,
    Turn(Direction),
    /// The player pressed (true) or let go of (false) the brake.
    Brake(bool),
    Emote(Emote),
}

impl ArenaInput {
    pub fn process_into_updates(self, arena: &mut Arena, player_id: PlayerId) -> Vec<ArenaUpdate> {
        match self {
            ArenaInput::Start => {
                // A round can be cut short once only NPCs are left in it.
                if arena.started.is_some() && arena.any_human_alive() {
                    trace!(
                        "Refusing to start arena that is already started: {}",
                        arena.id
                    );
                    return vec![];
                }
                // Forget an NPC winner of a round cut short, so the new
                // round isn't ended when that winner's timeout runs out.
                arena.winner_at = None;
                arena.queued_turns.clear();

                let mut updates = Vec::with_capacity(
                    1 + arena.lightcycles.len()
                        + arena.lightribbons.len()
                        + (arena.players.len() * 2)
                        + 1,
                );

                // remove existing winner
                updates.push(ArenaUpdate::SetWinner(None));

                // remove existing lightcycles
                arena
                    .lightcycles
                    .keys()
                    .copied()
                    .for_each(|id| updates.push(ArenaUpdate::RemoveLightcycle(id)));

                // remove existing lightribbons
                arena
                    .lightribbons
                    .keys()
                    .copied()
                    .for_each(|id| updates.push(ArenaUpdate::RemoveLightribbon(id)));

                // replace the last round's NPC players, which stay until now
                // so the end of the round can still show their names
                let human_ids: Vec<PlayerId> = arena.human_player_ids().collect();
                for (npc_id, _) in arena.npc_states.drain() {
                    updates.push(ArenaUpdate::RemovePlayer(npc_id));
                }
                arena.unrecorded_round_players.extend(&human_ids);
                let npc_count = npc::npc_count_for_arena(human_ids.len());
                let npc_players = npc::create_npc_players(npc_count);
                let npc_styles = npc::Style::for_round(npc_players.len());
                for (npc_player, style) in npc_players.iter().zip(npc_styles) {
                    arena
                        .npc_states
                        .insert(npc_player.id, npc::NpcState::new(style));
                    updates.push(ArenaUpdate::AddPlayer(npc_player.id, npc_player.clone()));
                }

                let round = Round::roll();
                let speed = LIGHTCYCLE_SPEED * round.speed_scale();
                updates.push(ArenaUpdate::SetRound(round));

                // add new lightcycles and lightribbons
                let mut player_ids = human_ids;
                player_ids.extend(npc_players.iter().map(|p| p.id));
                calculate_spawnpoints(player_ids).drain(..).for_each(
                    |(player_id, spawn_position, spawn_direction)| {
                        updates.push(ArenaUpdate::AddLightcycle(
                            player_id,
                            Lightcycle {
                                position: spawn_position,
                                direction: spawn_direction,
                                speed,
                                ..Default::default()
                            },
                        ));
                        updates.push(ArenaUpdate::AddLightribbon(
                            player_id,
                            Lightribbon::new(spawn_position),
                        ));
                    },
                );

                // begin countdown
                updates.push(ArenaUpdate::Start(
                    Utc::now() + OldDuration::seconds(ARENA_START_TIMER_SECONDS),
                ));

                updates
            }

            ArenaInput::Turn(direction) => {
                let lightcycle = match arena.lightcycles.get(&player_id) {
                    Some(lightcycle) => lightcycle,
                    None => {
                        error!("Lightcycle {} not found", player_id);
                        return vec![];
                    }
                };

                match arena.started {
                    None => {
                        trace!("Refusing to turn lightcycle in stopped arena");
                        return vec![];
                    }
                    Some(started) => {
                        let earliest = started - OldDuration::milliseconds(EARLY_TURN_MILLISECONDS);
                        if Utc::now() < earliest {
                            trace!("Refusing to turn lightcycle before arena has started");
                            return vec![];
                        }
                    }
                }

                if lightcycle.dead {
                    trace!("Refusing to turn dead lightcycle");
                    return vec![];
                }

                // Turns are checked against the last one queued, which is
                // the direction the lightcycle will be going when it's taken.
                let queued = arena.queued_turns.get(&player_id);
                let last_direction = queued
                    .and_then(|turns| turns.back().copied())
                    .unwrap_or(lightcycle.direction);

                if last_direction.is_opposite(direction) {
                    trace!("Refusing to turn lightcycle in opposite direction");
                    return vec![];
                }

                if last_direction == direction {
                    trace!("Refusing to turn lightcycle in the direction it is already going");
                    return vec![];
                }

                if queued.is_some_and(|turns| turns.len() >= MAX_QUEUED_TURNS) {
                    trace!("Refusing to queue another turn for lightcycle");
                    return vec![];
                }

                arena
                    .queued_turns
                    .entry(player_id)
                    .or_default()
                    .push_back(direction);
                vec![]
            }

            ArenaInput::Brake(braking) => {
                let Some(lightcycle) = arena.lightcycles.get(&player_id) else {
                    return vec![];
                };
                if arena.started.is_none() || lightcycle.dead || lightcycle.braking == braking {
                    return vec![];
                }
                vec![ArenaUpdate::UpdateLightcycleBraking(player_id, braking)]
            }

            ArenaInput::Emote(emote) => {
                if !arena.players.contains_key(&player_id) || !arena.take_emote_turn(player_id) {
                    trace!("Refusing emote from {}", player_id);
                    return vec![];
                }
                vec![ArenaUpdate::Emote(player_id, emote)]
            }
        }
    }
}
