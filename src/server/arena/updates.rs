use super::*;

#[derive(Debug, Clone, Hash, PartialEq, Serialize)]
pub enum ArenaUpdate {
    AddPlayer(PlayerId, Player),
    AddLightcycle(PlayerId, Lightcycle),
    AddLightribbon(PlayerId, Lightribbon),

    Start(DateTime<Utc>),
    End,

    UpdateLightcyclePosition(PlayerId, ArenaPoint),
    UpdateLightcycleDirection(PlayerId, Direction),
    UpdateLightcycleApplyDeath(PlayerId),

    UpdateLightribbonAppendPoint(PlayerId, ArenaPoint),
    UpdateLightribbonReplaceLatestPoint(PlayerId, ArenaPoint),

    RemovePlayer(PlayerId),
    RemoveLightcycle(PlayerId),
    RemoveLightribbon(PlayerId),
}

impl ArenaUpdate {
    pub fn apply<'s, 'arena>(&'s self, arena: &'arena mut Arena) -> &'arena mut Arena {
        match self {
            ArenaUpdate::AddPlayer(player_id, player) => {
                arena.players.insert(*player_id, player.clone());
            }
            ArenaUpdate::AddLightcycle(player_id, lightcycle) => {
                arena.lightcycles.insert(*player_id, *lightcycle);
            }
            ArenaUpdate::AddLightribbon(player_id, lightribbon) => {
                arena.lightribbons.insert(*player_id, lightribbon.clone());
            }

            ArenaUpdate::Start(start_at) => arena.started = Some(*start_at),
            ArenaUpdate::End => arena.started = None,

            ArenaUpdate::UpdateLightcyclePosition(player_id, position) => {
                let lightcycle = match arena.lightcycles.get_mut(player_id) {
                    Some(lightcycle) => lightcycle,
                    None => {
                        error!("Lightcycle {} not found", player_id);
                        return arena;
                    }
                };

                lightcycle.position = *position;
            }
            ArenaUpdate::UpdateLightcycleDirection(player_id, direction) => {
                let lightcycle = match arena.lightcycles.get_mut(player_id) {
                    Some(lightcycle) => lightcycle,
                    None => {
                        error!("Lightcycle {} not found", player_id);
                        return arena;
                    }
                };

                lightcycle.direction = *direction;
            }
            ArenaUpdate::UpdateLightcycleApplyDeath(player_id) => {
                let lightcycle = match arena.lightcycles.get_mut(player_id) {
                    Some(lightcycle) => lightcycle,
                    None => {
                        error!("Lightcycle {} not found", player_id);
                        return arena;
                    }
                };

                lightcycle.dead = true;
            }

            ArenaUpdate::UpdateLightribbonAppendPoint(player_id, point) => {
                let lightribbon = match arena.lightribbons.get_mut(player_id) {
                    Some(lightribbon) => lightribbon,
                    None => {
                        error!("Lightribbon {} not found", player_id);
                        return arena;
                    }
                };

                lightribbon.points.push(*point);
            }
            ArenaUpdate::UpdateLightribbonReplaceLatestPoint(player_id, latest_point) => {
                let lightribbon = match arena.lightribbons.get_mut(player_id) {
                    Some(lightribbon) => lightribbon,
                    None => {
                        error!("Lightribbon {} not found", player_id);
                        return arena;
                    }
                };

                lightribbon.points.pop();
                lightribbon.points.push(*latest_point);
            }

            ArenaUpdate::RemovePlayer(player_id) => {
                arena.players.remove(player_id);
            }
            ArenaUpdate::RemoveLightcycle(player_id) => {
                arena.lightcycles.remove(player_id);
            }
            ArenaUpdate::RemoveLightribbon(player_id) => {
                arena.lightribbons.remove(player_id);
            }
        }

        arena
    }
}
