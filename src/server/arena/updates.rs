use super::*;

#[derive(Debug, Clone, Hash, PartialEq, Serialize)]
pub enum ArenaUpdate {
    AddPlayer(PlayerId, Player),
    AddLightcycle(PlayerId, Lightcycle),
    AddTrail(PlayerId, Trail),

    Start(DateTime<Utc>),
    End,

    UpdateLightcyclePosition(PlayerId, ArenaPoint),
    UpdateLightcycleDirection(PlayerId, Direction),
    UpdateLightcycleApplyDeath(PlayerId),

    UpdateTrailAppendPoint(PlayerId, ArenaPoint),
    UpdateTrailReplaceLatestPoint(PlayerId, ArenaPoint),

    RemovePlayer(PlayerId),
    RemoveLightcycle(PlayerId),
    RemoveTrail(PlayerId),
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
            ArenaUpdate::AddTrail(player_id, trail) => {
                arena.trails.insert(*player_id, trail.clone());
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

            ArenaUpdate::UpdateTrailAppendPoint(player_id, point) => {
                let trail = match arena.trails.get_mut(player_id) {
                    Some(trail) => trail,
                    None => {
                        error!("Trail {} not found", player_id);
                        return arena;
                    }
                };

                trail.points.push(*point);
            }
            ArenaUpdate::UpdateTrailReplaceLatestPoint(player_id, latest_point) => {
                let trail = match arena.trails.get_mut(player_id) {
                    Some(trail) => trail,
                    None => {
                        error!("Trail {} not found", player_id);
                        return arena;
                    }
                };

                trail.points.pop();
                trail.points.push(*latest_point);
            }

            ArenaUpdate::RemovePlayer(player_id) => {
                arena.players.remove(player_id);
            }
            ArenaUpdate::RemoveLightcycle(player_id) => {
                arena.lightcycles.remove(player_id);
            }
            ArenaUpdate::RemoveTrail(player_id) => {
                arena.trails.remove(player_id);
            }
        }

        arena
    }
}
