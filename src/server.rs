mod arena;
mod messages;
mod primitives;

use anyhow::{anyhow, Context as ResultContext, Error};
use log::{error, info, warn};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time;

pub use arena::{Arena, ArenaInput, ArenaOverview, ArenaUpdate};
pub use messages::{MessageIn, MessageOut};
pub use primitives::*;

use crate::get_error_chain;
use messages::MessageInPayload;

const UPDATE_RATE_MILLISECONDS: u64 = 25; // 1000 / 25 = 40 updates per second

#[derive(Debug)]
pub struct Server {
    message_queue: Receiver<MessageIn>,
    clients: HashMap<ClientId, Client>,
    arenas: HashMap<ArenaId, Arena>,
}

impl Server {
    pub fn new(message_queue: Receiver<MessageIn>) -> Self {
        Self {
            message_queue,
            clients: Default::default(),
            arenas: Default::default(),
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

            // discard arena if all players have left
            !arena.players.is_empty()
        })
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

                if arena.players.len() >= arena.max_players {
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
