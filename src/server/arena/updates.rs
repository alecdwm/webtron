use super::*;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ArenaUpdate {
    AddPlayer(PlayerId, Player),
    AddLightcycle(PlayerId, Lightcycle),
    AddLightribbon(PlayerId, Lightribbon),
    AddHole(Hole),

    /// The layout and rules of the next round, set just before it starts.
    SetRound(Round),
    Start(DateTime<Utc>),
    End,
    SetWinner(Option<PlayerId>),

    UpdateLightcyclePosition(PlayerId, ArenaPoint),
    UpdateLightcycleDirection(PlayerId, Direction),
    UpdateLightcycleSpeed(PlayerId, f64),
    UpdateLightcycleApplyDeath(PlayerId, DeathCause),
    UpdateLightcycleBraking(PlayerId, bool),
    UpdateLightcycleBrakeMeter(PlayerId, f64),

    UpdateLightribbonAppendPoint(PlayerId, ArenaPoint),
    UpdateLightribbonReplaceLatestPoint(PlayerId, ArenaPoint),
    /// Starts a new path of the trail at the point, where a lightcycle comes
    /// back into the arena after riding off the opposite edge.
    UpdateLightribbonStartPath(PlayerId, ArenaPoint),
    /// Removes this length from the oldest end of the trail.
    TrimLightribbon(PlayerId, f64),

    /// A lightcycle came close to crashing at the point, and got some brake
    /// back for it. The flag is set when it was about as close as it could
    /// have come. Clients show it; it changes nothing.
    NearMiss(PlayerId, ArenaPoint, bool),
    /// A player flashed up an emote. Clients show it; it changes nothing.
    Emote(PlayerId, Emote),

    RemovePlayer(PlayerId),
    RemoveLightcycle(PlayerId),
    RemoveLightribbon(PlayerId),
    /// Removes a dead lightcycle's trail. Clients animate it disappearing.
    DerezLightribbon(PlayerId),
}

impl ArenaUpdate {
    pub fn apply<'arena>(&self, arena: &'arena mut Arena) -> &'arena mut Arena {
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

            ArenaUpdate::SetRound(round) => {
                arena.walls = round.walls(arena.width, arena.height);
                arena.portals = round.portal_rects(arena.width, arena.height);
                arena.round = round.clone();
            }
            ArenaUpdate::Start(start_at) => {
                arena.started = Some(*start_at);
                arena.holes.clear();
                arena.departed.clear();
                arena.near_miss_at.clear();
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
            ArenaUpdate::UpdateLightcycleBraking(player_id, braking) => {
                if let Some(lightcycle) = lightcycle_mut(arena, player_id) {
                    lightcycle.braking = *braking;
                }
            }
            ArenaUpdate::UpdateLightcycleBrakeMeter(player_id, brake) => {
                if let Some(lightcycle) = lightcycle_mut(arena, player_id) {
                    lightcycle.brake = *brake;
                }
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
            ArenaUpdate::UpdateLightribbonStartPath(player_id, point) => {
                if let Some(lightribbon) = lightribbon_mut(arena, player_id) {
                    lightribbon.start_path(*point);
                }
            }
            ArenaUpdate::TrimLightribbon(player_id, length) => {
                if let Some(lightribbon) = lightribbon_mut(arena, player_id) {
                    lightribbon.trim_front(*length);
                }
            }

            ArenaUpdate::NearMiss(..) | ArenaUpdate::Emote(..) => {}

            ArenaUpdate::RemovePlayer(player_id) => {
                if let Some(player) = arena.players.remove(player_id)
                    && !player.npc
                    && arena.lightcycles.contains_key(player_id)
                {
                    arena.departed.insert(*player_id, player);
                }
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

fn lightcycle_mut<'arena>(
    arena: &'arena mut Arena,
    player_id: &PlayerId,
) -> Option<&'arena mut Lightcycle> {
    let lightcycle = arena.lightcycles.get_mut(player_id);
    if lightcycle.is_none() {
        error!("Lightcycle {} not found", player_id);
    }
    lightcycle
}

fn lightribbon_mut<'arena>(
    arena: &'arena mut Arena,
    player_id: &PlayerId,
) -> Option<&'arena mut Lightribbon> {
    let lightribbon = arena.lightribbons.get_mut(player_id);
    if lightribbon.is_none() {
        error!("Lightribbon {} not found", player_id);
    }
    lightribbon
}
