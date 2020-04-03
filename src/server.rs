mod arena;
mod messages;
mod primitives;

use actix::{Actor, AsyncContext, Context, Handler};
use anyhow::{anyhow, Context as ResultContext, Error};
use log::{error, info, warn};
use std::collections::HashMap;
use std::time::Duration;

pub use arena::{Arena, ArenaInput, ArenaOverview, ArenaUpdate};
pub use messages::{MessageIn, MessageOut};
pub use primitives::*;

use crate::get_error_chain;
use messages::MessageInPayload;

const UPDATE_RATE_MILLISECONDS: u64 = 25; // 1000 / 25 = 40 updates per second

#[derive(Debug, Default)]
pub struct Server {
    message_queue: Vec<MessageIn>,
    clients: HashMap<ClientId, Client>,
    arenas: HashMap<ArenaId, Arena>,
}

impl Server {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_arena(&mut self) -> ArenaId {
        let arena = Arena::default();
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
            .with_context(|| anyhow!("Client {} not in an arena"))?;

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
            .with_context(|| anyhow!("Client {} not in an arena"))?;

        let arena = self
            .arenas
            .get_mut(&arena_id)
            .with_context(|| anyhow!("Arena {} not found", arena_id))?;

        let player_id = client
            .player
            .with_context(|| anyhow!("Client {} has no player", client_id))?;

        arena.remove_player(player_id);

        // TODO: Remove empty arenas

        Ok(())
    }
}

impl Server {
    pub fn handle_message(
        &mut self,
        client_id: ClientId,
        payload: MessageInPayload,
    ) -> Result<(), Error> {
        match payload {
            MessageInPayload::Connect(ip_address, address) => {
                info!("Client connected: {}", client_id);
                self.clients.insert(
                    client_id,
                    Client {
                        id: client_id,
                        ip_address,
                        address,
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
                    .address
                    .try_send(MessageOut::ArenaList(arena_list))
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
                            self.new_arena()
                        }
                    }
                    None => self.new_arena(),
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
                    .address
                    .try_send(MessageOut::ArenaJoined(arena.id))
                    .with_context(|| {
                        anyhow!("Failed to send ArenaJoined to client {}", client_id)
                    })?;

                info!("Player {} joined arena {}", player_id, arena_id);
            }

            MessageInPayload::Start => {
                self.client_input(client_id, ArenaInput::Start)?;
                info!("Client {} started game", client_id);
            }
            MessageInPayload::Turn(direction) => {
                self.client_input(client_id, ArenaInput::Turn(direction))?;
            }
        }
        Ok(())
    }
}

impl Server {
    pub fn process_messages(&mut self) {
        // take ownership of queued messages
        let mut process_messages_queue = Vec::with_capacity(self.message_queue.len());

        // move all message_queue messages into process_messages_queue
        process_messages_queue.append(&mut self.message_queue);

        // process each message
        process_messages_queue.drain(..).for_each(|message| {
            self.handle_message(message.client_id, message.payload)
                .unwrap_or_else(|error| {
                    error!("Failed processing MessageIn: {}", get_error_chain(error))
                });
        });
    }

    pub fn update(&mut self) {
        self.arenas.values_mut().for_each(|arena| arena.update());
    }

    pub fn send_updates(&mut self) {
        let clients = &mut self.clients;
        let arenas = &self.arenas;

        clients.values_mut().for_each(|client| {
            let arena = match client.arena {
                Some(arena) => arena,
                None => return,
            };

            let arena = match arenas.get(&arena) {
                Some(arena) => arena,
                None => return,
            };

            if client.updates_sent_so_far == 0 {
                unwrap_or_return!(
                    client
                        .address
                        .try_send(MessageOut::ArenaState(Box::from(arena.clone()))),
                    |error| error!("Failed to send ArenaState to client: {}", error)
                );
                client.updates_sent_so_far = arena.updates.len();
                return;
            }

            if client.updates_sent_so_far < arena.updates.len() {
                client
                    .address
                    .try_send(MessageOut::ArenaStatePatch(
                        arena
                            .updates
                            .iter()
                            .skip(client.updates_sent_so_far)
                            .cloned()
                            .collect(),
                    ))
                    .unwrap_or_else(|error| {
                        error!("Failed to send ArenaStatePatch to client: {}", error);
                    });
                client.updates_sent_so_far = arena.updates.len();
                return;
            }
        })
    }
}

impl Actor for Server {
    type Context = Context<Self>;

    fn started(&mut self, context: &mut Context<Self>) {
        context.run_interval(
            Duration::from_millis(UPDATE_RATE_MILLISECONDS),
            |server, _context| {
                server.process_messages();
                server.update();
                server.send_updates();
            },
        );
    }
}

impl Handler<MessageIn> for Server {
    type Result = ();

    fn handle(&mut self, message: MessageIn, _context: &mut Context<Self>) {
        self.message_queue.push(message);
    }
}
