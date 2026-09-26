mod arena;
mod leaderboard;
mod messages;
mod primitives;

use anyhow::{Context as ResultContext, Error, anyhow};
use log::{error, info, warn};
use std::collections::HashMap;
use std::mem;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time;

pub use arena::{Arena, ArenaInput, ArenaOverview, ArenaUpdate, DeathCause};
pub use leaderboard::{Leaderboard, LeaderboardStandings, Stat};
pub use messages::{MessageIn, MessageOut};
pub use primitives::*;

use crate::get_error_chain;
use leaderboard::normalize_name;
use messages::MessageInPayload;

const UPDATE_RATE_MILLISECONDS: u64 = 50; // 1000 / 50 = 20 updates per second

#[derive(Debug)]
pub struct Server {
    message_queue: Receiver<MessageIn>,
    clients: HashMap<ClientId, Client>,
    arenas: HashMap<ArenaId, Arena>,
    leaderboard: Leaderboard,
}

impl Server {
    pub fn new(message_queue: Receiver<MessageIn>, leaderboard: Leaderboard) -> Self {
        Self {
            message_queue,
            clients: Default::default(),
            arenas: Default::default(),
            leaderboard,
        }
    }

    pub async fn start(mut self) {
        let mut interval = time::interval(Duration::from_millis(UPDATE_RATE_MILLISECONDS));
        loop {
            interval.tick().await;
            self.process_messages().await;
            self.update(UPDATE_RATE_MILLISECONDS as f64 / 1000.0);
            self.send_updates().await;
        }
    }

    pub async fn process_messages(&mut self) {
        while let Ok(message) = self.message_queue.try_recv() {
            self.handle_message(message.client_id, message.payload)
                .await
                .unwrap_or_else(|error| {
                    error!(
                        "Failed to process incoming message: {}",
                        get_error_chain(error)
                    )
                })
        }
    }

    pub fn update(&mut self, delta_time: f64) {
        self.arenas.retain(|_, arena| {
            arena.update(delta_time);

            // discard arena if all human players have left
            arena.human_player_ids().next().is_some()
        });

        let ids: Vec<ArenaId> = self.arenas.keys().copied().collect();
        for arena_id in ids {
            self.record_leaderboard_events(arena_id);
        }
    }

    /// Adds this update's rounds, wins, kills and deaths of human players to
    /// the leaderboard. If the round was won, sends the leaderboard to everyone
    /// in the arena. The leaderboard file is used off the game loop.
    fn record_leaderboard_events(&mut self, arena_id: ArenaId) {
        let Some(arena) = self.arenas.get_mut(&arena_id) else {
            return;
        };
        let winner = arena.unrecorded_winner.take();
        let deaths = mem::take(&mut arena.unrecorded_deaths);
        let round_players = mem::take(&mut arena.unrecorded_round_players);
        let arena = &*arena;

        let human_name = |id: &PlayerId| human_name(arena, id);

        let mut events: Vec<(String, Stat)> = Vec::new();
        for player in round_players {
            if let Some(name) = human_name(&player) {
                events.push((name, Stat::RoundPlayed));
            }
        }
        for (victim, cause) in deaths {
            events.extend(death_events(arena, victim, cause));
        }
        if let Some(winner) = winner {
            if let (Some(name), Some(player)) = (human_name(&winner), arena.players.get(&winner)) {
                events.push((name, Stat::Win(player.color)));
            }
        }

        if events.is_empty() && winner.is_none() {
            return;
        }

        // Only a won round shows the leaderboard.
        let recipients: Vec<(Sender<MessageOut>, Option<String>)> = match winner {
            Some(_) => self
                .clients
                .values()
                .filter(|client| client.arena == Some(arena.id))
                .map(|client| {
                    let name = client
                        .player
                        .and_then(|id| arena.players.get(&id))
                        .map(|player| player.name.clone());
                    (client.tx.clone(), name)
                })
                .collect(),
            None => Vec::new(),
        };

        let leaderboard = self.leaderboard.clone();
        tokio::spawn(async move {
            let standings = tokio::task::spawn_blocking(move || {
                let stats = leaderboard.record(&events)?;
                Ok::<_, Error>(
                    recipients
                        .into_iter()
                        .map(|(tx, name)| (tx, stats.standings(name.as_deref())))
                        .collect::<Vec<_>>(),
                )
            })
            .await;

            let standings = match standings {
                Ok(Ok(standings)) => standings,
                Ok(Err(error)) => {
                    return error!("Failed to update leaderboard: {}", get_error_chain(error));
                }
                Err(error) => return error!("Failed to update leaderboard: {}", error),
            };

            for (tx, standings) in standings {
                if let Err(error) = tx.send(MessageOut::Leaderboard(standings)).await {
                    warn!("Failed to send Leaderboard to client: {}", error);
                }
            }
        });
    }

    pub async fn send_updates(&mut self) {
        let clients = &mut self.clients;
        let arenas = &self.arenas;

        for client in clients.values_mut() {
            let arena = match client.arena {
                Some(arena) => arena,
                None => continue,
            };

            let arena = match arenas.get(&arena) {
                Some(arena) => arena,
                None => continue,
            };

            if client.updates_sent_so_far == 0 {
                if let Err(error) = client
                    .tx
                    .send(MessageOut::ArenaState(Box::from(arena.clone())))
                    .await
                {
                    error!("Failed to send ArenaState to client: {}", error);
                    continue;
                }
                client.updates_sent_so_far = arena.updates.len();
                continue;
            }

            if client.updates_sent_so_far < arena.updates.len() {
                if let Err(error) = client
                    .tx
                    .send(MessageOut::ArenaStatePatch(
                        arena
                            .updates
                            .iter()
                            .skip(client.updates_sent_so_far)
                            .cloned()
                            .collect(),
                    ))
                    .await
                {
                    error!("Failed to send ArenaStatePatch to client: {}", error);
                    continue;
                }
                client.updates_sent_so_far = arena.updates.len();
            }
        }
    }
}

impl Server {
    /// Claims `name` for the client, unless another connected client has
    /// already claimed it, in which case the client is told so. Names are
    /// compared the way the leaderboard compares them. Returns whether the
    /// claim succeeded.
    async fn claim_name(&mut self, client_id: ClientId, name: &str) -> Result<bool, Error> {
        let name = normalize_name(name);
        let in_use = self
            .clients
            .values()
            .any(|client| client.id != client_id && client.name.as_ref() == Some(&name));

        let client = self
            .clients
            .get_mut(&client_id)
            .ok_or_else(|| anyhow!("Client {} not found", client_id))?;

        if in_use {
            client
                .tx
                .send(MessageOut::NameInUse)
                .await
                .with_context(|| anyhow!("Failed to send NameInUse to client {}", client_id))?;
            info!("Client {} tried to claim a name in use", client_id);
            return Ok(false);
        }

        client.name = Some(name);
        Ok(true)
    }

    pub fn new_arena(&mut self, name: &str) -> ArenaId {
        let arena = Arena::with_name(name);
        let id = arena.id;

        self.arenas.insert(id, arena);

        id
    }

    pub fn client_input(&mut self, client_id: ClientId, input: ArenaInput) -> Result<(), Error> {
        let client = self
            .clients
            .get(&client_id)
            .with_context(|| anyhow!("Client {} not found", client_id))?;

        let arena_id = client
            .arena
            .with_context(|| anyhow!("Client {} not in an arena", client_id))?;

        let arena = self
            .arenas
            .get_mut(&arena_id)
            .with_context(|| anyhow!("Arena {} not found", arena_id))?;

        let player_id = client
            .player
            .with_context(|| anyhow!("Client {} has no player", client_id))?;

        arena.process_input(player_id, input);

        Ok(())
    }

    pub fn client_part_arena(&mut self, client_id: ClientId) -> Result<(), Error> {
        let client = self
            .clients
            .get_mut(&client_id)
            .with_context(|| anyhow!("Client {} not found", client_id))?;

        client.updates_sent_so_far = 0;

        let arena_id = client
            .arena
            .with_context(|| anyhow!("Client {} not in an arena", client_id))?;

        let arena = self
            .arenas
            .get_mut(&arena_id)
            .with_context(|| anyhow!("Arena {} not found", arena_id))?;

        let player_id = client
            .player
            .with_context(|| anyhow!("Client {} has no player", client_id))?;

        arena.remove_player(player_id);

        Ok(())
    }
}

impl Server {
    pub async fn handle_message(
        &mut self,
        client_id: ClientId,
        payload: MessageInPayload,
    ) -> Result<(), Error> {
        match payload {
            MessageInPayload::Connect(ip_address, tx) => {
                info!("Client connected: {}", client_id);
                self.clients.insert(
                    client_id,
                    Client {
                        id: client_id,
                        ip_address,
                        tx,
                        player: None,
                        arena: None,
                        updates_sent_so_far: 0,
                        name: None,
                    },
                );
            }
            MessageInPayload::Disconnect => {
                info!("Client disconnected: {}", client_id);

                self.client_part_arena(client_id).unwrap_or_else(|error| {
                    warn!("Failed to remove client from their arena: {}", error)
                });

                self.clients
                    .remove(&client_id)
                    .ok_or_else(|| anyhow!("Failed to remove client {}", client_id))?;
            }

            MessageInPayload::ClaimName(name) => {
                self.claim_name(client_id, &name).await?;
            }
            MessageInPayload::GetArenaList => {
                let arena_list: Vec<ArenaOverview> =
                    self.arenas.values().map(ArenaOverview::from).collect();

                let client = self
                    .clients
                    .get_mut(&client_id)
                    .ok_or_else(|| anyhow!("Client {} not found", client_id))?;

                client
                    .tx
                    .send(MessageOut::ArenaList(arena_list))
                    .await
                    .with_context(|| anyhow!("Failed to send ArenaList to client {}", client_id))?;
            }
            MessageInPayload::Join { player, arena_id } => {
                self.client_part_arena(client_id).unwrap_or_else(|error| {
                    warn!("Failed to remove client from their arena: {}", error)
                });

                if !self.claim_name(client_id, &player.name).await? {
                    return Ok(());
                }

                let arena_id = match arena_id {
                    Some(arena_id) => {
                        if self.arenas.contains_key(&arena_id) {
                            arena_id
                        } else {
                            self.new_arena(&player.name)
                        }
                    }
                    None => self.new_arena(&player.name),
                };

                let arena = self
                    .arenas
                    .get_mut(&arena_id)
                    .ok_or_else(|| anyhow!("Arena {} not found", arena_id))?;

                if arena.human_player_ids().count() >= arena.max_players {
                    return Err(anyhow!("Arena {} is full", arena_id));
                }

                let player_id = player.id;

                let client = self
                    .clients
                    .get_mut(&client_id)
                    .ok_or_else(|| anyhow!("Client {} not found", client_id))?;

                arena.add_player(player);
                client.player = Some(player_id);
                client.arena = Some(arena_id);

                client
                    .tx
                    .send(MessageOut::ArenaJoined(arena.id, player_id))
                    .await
                    .with_context(|| {
                        anyhow!("Failed to send ArenaJoined to client {}", client_id)
                    })?;

                info!("Player {} joined arena {}", player_id, arena_id);
            }

            MessageInPayload::Start => {
                self.client_input(client_id, ArenaInput::Start)?;
            }
            MessageInPayload::Turn(direction) => {
                self.client_input(client_id, ArenaInput::Turn(direction))?;
            }
        }
        Ok(())
    }
}

/// The name of a human player in the arena, or `None` for an NPC or a player
/// who has left.
fn human_name(arena: &Arena, id: &PlayerId) -> Option<String> {
    arena
        .players
        .get(id)
        .filter(|_| !arena.npc_states.contains_key(id))
        .map(|player| player.name.clone())
}

/// The leaderboard events for a lightcycle's death: a kill for the human
/// player whose trail it hit, and the kind of death for a human victim.
fn death_events(arena: &Arena, victim: PlayerId, cause: DeathCause) -> Vec<(String, Stat)> {
    let victim_name = human_name(arena, &victim);
    let owner = match cause {
        DeathCause::Boundary => {
            return victim_name
                .map(|name| (name, Stat::ArenaDeath))
                .into_iter()
                .collect();
        }
        DeathCause::Lightcycle => {
            return victim_name
                .map(|name| (name, Stat::HeadonCrash))
                .into_iter()
                .collect();
        }
        DeathCause::Trail(owner) => owner,
    };
    if owner == victim {
        return victim_name
            .map(|name| (name, Stat::Suicide))
            .into_iter()
            .collect();
    }

    let mut events = Vec::new();
    if let Some(name) = human_name(arena, &owner) {
        let kill = if arena.npc_states.contains_key(&victim) {
            Stat::NpcKill
        } else {
            Stat::PlayerKill
        };
        events.push((name, kill));
    }
    if let Some(name) = victim_name {
        let death = if arena.npc_states.contains_key(&owner) {
            Stat::NpcDeath
        } else {
            Stat::PlayerDeath
        };
        events.push((name, death));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(arena: &mut Arena, name: &str, npc: bool) -> PlayerId {
        let id = PlayerId::default();
        arena.players.insert(
            id,
            Player {
                id,
                name: name.to_string(),
                npc,
                ..Default::default()
            },
        );
        if npc {
            arena.npc_states.insert(id, Default::default());
        }
        id
    }

    fn events(list: &[(&str, Stat)]) -> Vec<(String, Stat)> {
        list.iter()
            .map(|(name, stat)| (name.to_string(), *stat))
            .collect()
    }

    #[test]
    fn crashing_into_your_own_trail_is_only_a_suicide() {
        let mut arena = Arena::default();
        let alec = player(&mut arena, "alec", false);
        assert_eq!(
            death_events(&arena, alec, DeathCause::Trail(alec)),
            events(&[("alec", Stat::Suicide)])
        );
    }

    #[test]
    fn crashes_into_other_trails_count_for_both_players() {
        let mut arena = Arena::default();
        let alec = player(&mut arena, "alec", false);
        let bob = player(&mut arena, "bob", false);
        let clu = player(&mut arena, "CLU", true);

        assert_eq!(
            death_events(&arena, bob, DeathCause::Trail(alec)),
            events(&[("alec", Stat::PlayerKill), ("bob", Stat::PlayerDeath)])
        );
        assert_eq!(
            death_events(&arena, clu, DeathCause::Trail(alec)),
            events(&[("alec", Stat::NpcKill)])
        );
        assert_eq!(
            death_events(&arena, alec, DeathCause::Trail(clu)),
            events(&[("alec", Stat::NpcDeath)])
        );
        assert_eq!(death_events(&arena, clu, DeathCause::Trail(clu)), vec![]);
        assert_eq!(
            death_events(&arena, alec, DeathCause::Boundary),
            events(&[("alec", Stat::ArenaDeath)])
        );
        assert_eq!(
            death_events(&arena, bob, DeathCause::Lightcycle),
            events(&[("bob", Stat::HeadonCrash)])
        );
    }
}
