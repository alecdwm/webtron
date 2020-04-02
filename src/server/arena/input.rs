use super::*;

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum ArenaInput {
    Start,
    Turn(Direction),
}

impl ArenaInput {
    pub fn process_into_updates(self, arena: &Arena, player_id: PlayerId) -> Vec<ArenaUpdate> {
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
                    arena.lightcycles.len() + arena.trails.len() + (arena.players.len() * 2) + 1,
                );

                // remove existing lightcycles
                arena
                    .lightcycles
                    .keys()
                    .copied()
                    .for_each(|id| updates.push(ArenaUpdate::RemoveLightcycle(id)));

                // remove existing trails
                arena
                    .trails
                    .keys()
                    .copied()
                    .for_each(|id| updates.push(ArenaUpdate::RemoveTrail(id)));

                // add new lightcycles and trails
                let player_ids = arena.players.keys().copied().collect();
                calculate_spawnpoints(player_ids).drain(..).for_each(
                    |(player_id, spawn_position, spawn_direction)| {
                        updates.push(ArenaUpdate::AddLightcycle(
                            player_id,
                            Lightcycle {
                                position: spawn_position,
                                rotation: spawn_direction,
                                ..Default::default()
                            },
                        ));
                        updates.push(ArenaUpdate::AddTrail(
                            player_id,
                            Trail {
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

                if lightcycle.rotation.is_opposite(direction) {
                    trace!("Refusing to turn lightcycle in opposite direction");
                    return vec![];
                }

                return vec![ArenaUpdate::UpdateLightcycleChangeDirection(
                    player_id, direction,
                )];
            }
        }
    }
}
