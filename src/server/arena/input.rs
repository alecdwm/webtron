use super::npc;
use super::*;

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum ArenaInput {
    Start,
    Turn(Direction),
}

impl ArenaInput {
    pub fn process_into_updates(self, arena: &mut Arena, player_id: PlayerId) -> Vec<ArenaUpdate> {
        match self {
            ArenaInput::Start => {
                if arena.started.is_some() {
                    trace!(
                        "Refusing to start arena that is already started: {}",
                        arena.id
                    );
                    return vec![];
                }

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

                // add NPC players
                arena.npc_states.clear();
                let npc_count = npc::npc_count_for_arena(arena.players.len());
                let npc_players = npc::create_npc_players(npc_count);
                for npc_player in &npc_players {
                    arena
                        .npc_states
                        .insert(npc_player.id, npc::NpcState::default());
                    updates.push(ArenaUpdate::AddPlayer(npc_player.id, npc_player.clone()));
                }

                // add new lightcycles and lightribbons
                let mut player_ids: Vec<PlayerId> = arena.players.keys().copied().collect();
                player_ids.extend(npc_players.iter().map(|p| p.id));
                calculate_spawnpoints(player_ids).drain(..).for_each(
                    |(player_id, spawn_position, spawn_direction)| {
                        updates.push(ArenaUpdate::AddLightcycle(
                            player_id,
                            Lightcycle {
                                position: spawn_position,
                                direction: spawn_direction,
                                ..Default::default()
                            },
                        ));
                        updates.push(ArenaUpdate::AddLightribbon(
                            player_id,
                            Lightribbon {
                                points: vec![spawn_position, spawn_position],
                            },
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
                        let now = Utc::now();
                        if now < started {
                            trace!("Refusing to turn lightcycle before arena has started");
                            return vec![];
                        }
                    }
                }

                if lightcycle.dead {
                    trace!("Refusing to turn dead lightcycle");
                    return vec![];
                }

                if lightcycle.direction.is_opposite(direction) {
                    trace!("Refusing to turn lightcycle in opposite direction");
                    return vec![];
                }

                return vec![
                    ArenaUpdate::UpdateLightribbonAppendPoint(player_id, lightcycle.position),
                    ArenaUpdate::UpdateLightcycleDirection(player_id, direction),
                ];
            }
        }
    }
}
