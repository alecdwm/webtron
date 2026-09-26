use super::*;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ArenaUpdate {
    AddPlayer(PlayerId, Player),
    AddLightcycle(PlayerId, Lightcycle),
    AddLightribbon(PlayerId, Lightribbon),
    AddHole(Hole),

    Start(DateTime<Utc>),
    End,
    SetWinner(Option<PlayerId>),

    UpdateLightcyclePosition(PlayerId, ArenaPoint),
    UpdateLightcycleDirection(PlayerId, Direction),
    UpdateLightcycleSpeed(PlayerId, f64),
    UpdateLightcycleApplyDeath(PlayerId, DeathCause),

    UpdateLightribbonAppendPoint(PlayerId, ArenaPoint),
    UpdateLightribbonReplaceLatestPoint(PlayerId, ArenaPoint),

    RemovePlayer(PlayerId),
    RemoveLightcycle(PlayerId),
    RemoveLightribbon(PlayerId),
    /// Removes a dead lightcycle's trail. Clients animate it disappearing.
    DerezLightribbon(PlayerId),
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

            ArenaUpdate::AddHole(hole) => {
                arena.holes.push(hole.clone());
            }

            ArenaUpdate::Start(start_at) => {
                arena.started = Some(*start_at);
                arena.holes.clear();
            }
            ArenaUpdate::End => arena.started = None,
            ArenaUpdate::SetWinner(winner) => arena.winner = *winner,

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
            ArenaUpdate::UpdateLightcycleSpeed(player_id, speed) => {
                let lightcycle = match arena.lightcycles.get_mut(player_id) {
                    Some(lightcycle) => lightcycle,
                    None => {
                        error!("Lightcycle {} not found", player_id);
                        return arena;
                    }
                };

                lightcycle.speed = *speed;
            }
            ArenaUpdate::UpdateLightcycleApplyDeath(player_id, cause) => {
                let lightcycle = match arena.lightcycles.get_mut(player_id) {
                    Some(lightcycle) => lightcycle,
                    None => {
                        error!("Lightcycle {} not found", player_id);
                        return arena;
                    }
                };

                lightcycle.dead = true;
                lightcycle.death_cause = Some(*cause);
                arena.unrecorded_deaths.push((*player_id, *cause));
            }

            ArenaUpdate::UpdateLightribbonAppendPoint(player_id, point) => {
                let lightribbon = match arena.lightribbons.get_mut(player_id) {
                    Some(lightribbon) => lightribbon,
                    None => {
                        error!("Lightribbon {} not found", player_id);
                        return arena;
                    }
                };

                lightribbon.append_point(*point);
            }
            ArenaUpdate::UpdateLightribbonReplaceLatestPoint(player_id, latest_point) => {
                let lightribbon = match arena.lightribbons.get_mut(player_id) {
                    Some(lightribbon) => lightribbon,
                    None => {
                        error!("Lightribbon {} not found", player_id);
                        return arena;
                    }
                };

                lightribbon.replace_latest_point(*latest_point);
            }

            ArenaUpdate::RemovePlayer(player_id) => {
                arena.players.remove(player_id);
            }
            ArenaUpdate::RemoveLightcycle(player_id) => {
                arena.lightcycles.remove(player_id);
            }
            ArenaUpdate::RemoveLightribbon(player_id)
            | ArenaUpdate::DerezLightribbon(player_id) => {
                arena.lightribbons.remove(player_id);
            }
        }

        arena
    }
}
