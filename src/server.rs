mod arena;
mod messages;
mod primitives;

use actix::{Actor, AsyncContext, Context, Handler, Recipient};
use chrono::{DateTime, Duration as OldDuration, Utc};
use debug_stub_derive::DebugStub;
use failure::{format_err, Error, ResultExt};
use log::{error, info};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

pub use arena::{Arena, ArenaUpdates};
pub use messages::incoming::{ConnectionMessage, GameInputMessage, MatchmakingMessage};
pub use messages::{MessageIn, MessageOut};
pub use primitives::*;

use crate::get_error_chain;
use messages::MessageInPayload;

const MAX_PLAYERS_PER_GAME: usize = 8;
const GAME_START_TIMER_SECONDS: i64 = 5;
const UPDATE_RATE_MILLISECONDS: u64 = 25; // 1000 / 25 = 40 updates per second

//
// Datastructure
//

#[derive(Debug, Default)]
pub struct Server {
    message_queue: Vec<MessageIn>,
    clients: HashMap<ClientId, Client>,
    players: HashMap<PlayerId, Player>,
    games: HashMap<GameId, Game>,
}

#[derive(DebugStub)]
pub struct Client {
    // data
    pub ip_address: Option<String>,
    #[debug_stub = "Recipient<MessageOut>"]
    pub address: Recipient<MessageOut>,

    // id + relations
    pub id: ClientId,
    pub player: Option<PlayerId>,
}

#[derive(Debug, Default, Clone)]
pub struct Player {
    // data
    pub name: String,
    pub color: PlayerColor,

    // id + relations
    pub id: PlayerId,
    pub client: Option<ClientId>,
    pub game: Option<GameId>,
}

#[derive(Debug, Clone)]
pub struct Game {
    // data
    pub max_players: usize,
    pub arena: Arena,
    pub started: Option<DateTime<Utc>>,

    // id + relations
    pub id: GameId,
    pub players: HashSet<PlayerId>,
}

//
// Implementations
//

impl Server {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_client(
        &mut self,
        id: ClientId,
        ip_address: Option<String>,
        address: Recipient<MessageOut>,
    ) {
        self.clients.insert(
            id,
            Client {
                ip_address,
                address,
                id,
                player: None,
            },
        );
    }

    pub fn configure_player(
        &mut self,
        client_id: ClientId,
        name: String,
        color: PlayerColor,
    ) -> Result<PlayerId, Error> {
        let client = self
            .clients
            .get_mut(&client_id)
            .ok_or_else(|| format_err!("Client not found: {}", client_id))?;

        if let Some(player_id) = client.player {
            let player = self
                .players
                .get_mut(&player_id)
                .ok_or_else(|| format_err!("Player not found: {}", player_id))?;

            player.name = name;
            player.color = color;

            return Ok(player.id);
        }

        let player_id = PlayerId::new_v4();
        self.players.insert(
            player_id,
            Player {
                name,
                color,

                id: player_id,
                client: Some(client.id),
                game: None,
            },
        );
        client.player = Some(player_id);

        Ok(player_id)
    }

    pub fn new_game(&mut self) -> GameId {
        let game_id = GameId::new_v4();

        self.games.insert(game_id, Game::with_id(game_id));

        game_id
    }

    pub fn remove_client(&mut self, id: ClientId) -> Result<(), Error> {
        let client = self
            .clients
            .remove(&id)
            .ok_or_else(|| format_err!("Client {} not found", id))?;

        self.remove_client_relations(client)
            .context("Failed to cleanup client relations")?;

        Ok(())
    }

    pub fn remove_player(&mut self, id: PlayerId) -> Result<(), Error> {
        let player = self
            .players
            .remove(&id)
            .ok_or_else(|| format_err!("Player {} not found", id))?;

        self.remove_player_relations(player)
            .context("Failed to cleanup player relations")?;

        Ok(())
    }

    pub fn remove_game(&mut self, id: GameId) -> Result<(), Error> {
        let game = self
            .games
            .remove(&id)
            .ok_or_else(|| format_err!("Game {} not found", id))?;

        self.remove_game_relations(game)
            .context("Failed to cleanup game relations")?;

        Ok(())
    }

    fn remove_client_relations(&mut self, client: Client) -> Result<(), Error> {
        if let Some(player_id) = client.player {
            let player = self
                .players
                .get_mut(&player_id)
                .ok_or_else(|| format_err!("Player {} not found", player_id))?;

            player.client = None;

            // when client is removed, also remove their player
            self.remove_player(player_id)
                .context("Failed to remove client's player")?;
        }

        Ok(())
    }

    fn remove_player_relations(&mut self, player: Player) -> Result<(), Error> {
        if let Some(client_id) = player.client {
            let client = self
                .clients
                .get_mut(&client_id)
                .ok_or_else(|| format_err!("Client {} not found", client_id))?;

            client.player = None;
        }

        if let Some(game_id) = player.game {
            let game = self
                .games
                .get_mut(&game_id)
                .ok_or_else(|| format_err!("Game {} not found", game_id))?;

            game.players.remove(&player.id);

            // when player is removed, also remove their game (if it is now empty)
            if game.is_empty() {
                self.remove_game(game_id)
                    .context("Failed to remove empty game")?;
            }
        }

        Ok(())
    }

    fn remove_game_relations(&mut self, game: Game) -> Result<(), Error> {
        for player_id in game.players {
            let player = self
                .players
                .get_mut(&player_id)
                .ok_or_else(|| format_err!("Player {} not found", player_id))?;

            player.game = None;
        }

        Ok(())
    }
}

impl Default for Game {
    fn default() -> Self {
        Self {
            max_players: MAX_PLAYERS_PER_GAME,
            arena: Default::default(),
            started: None,

            id: GameId::new_v4(),
            players: Default::default(),
        }
    }
}

impl Game {
    pub fn with_id(id: GameId) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn is_full(&self) -> bool {
        self.max_players <= self.players.len()
    }

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }
}

impl Server {
    pub fn message_client(
        &mut self,
        client_id: &ClientId,
        message: &MessageOut,
    ) -> Result<(), Error> {
        self.clients
            .get_mut(client_id)
            .ok_or_else(|| format_err!("Client not found: {}", client_id))?
            .address
            .try_send(message.clone())
            .with_context(|_| format_err!("Failed to message client {}", client_id))?;

        Ok(())
    }

    pub fn message_all_clients(&mut self, message: &MessageOut) {
        self.clients.values().for_each(|client| {
            client
                .address
                .try_send(message.clone())
                .unwrap_or_else(|error| error!("Failed to message client {}: {}", client.id, error))
        });
    }

    pub fn message_clients_in_game_from_client(
        &mut self,
        client_id: &ClientId,
        message: &MessageOut,
    ) -> Result<(), Error> {
        let player_id = self
            .clients
            .get(&client_id)
            .ok_or_else(|| format_err!("Client {} not found", client_id))?
            .player
            .ok_or_else(|| format_err!("Client {} has no player", client_id))?;

        self.message_clients_in_game_from_player(&player_id, message)
    }

    pub fn message_clients_in_game_from_player(
        &mut self,
        player_id: &PlayerId,
        message: &MessageOut,
    ) -> Result<(), Error> {
        let game_id = self
            .players
            .get(&player_id)
            .ok_or_else(|| format_err!("Player {} not found", player_id))?
            .game
            .ok_or_else(|| format_err!("Player {} not in a game", player_id))?;

        self.message_clients_in_game(&game_id, message)
    }

    pub fn message_clients_in_game(
        &mut self,
        game_id: &GameId,
        message: &MessageOut,
    ) -> Result<(), Error> {
        let game = self
            .games
            .get(&game_id)
            .ok_or_else(|| format_err!("Game {} not found", game_id))?;

        game.players
            .iter()
            .filter_map(|player_id| self.players.get(player_id))
            .filter_map(|player| player.client)
            .collect::<Vec<ClientId>>()
            .iter()
            .for_each(|client_id| {
                self.message_client(&client_id, &message)
                    .unwrap_or_else(|error| {
                        error!(
                            "Failed to send GameStarting message: {}",
                            get_error_chain(error)
                        )
                    })
            });

        Ok(())
    }

    pub fn client_join_game(
        &mut self,
        client_id: &ClientId,
        game_id: Option<GameId>,
    ) -> Result<GameId, Error> {
        let player_id = self
            .clients
            .get(&client_id)
            .ok_or_else(|| format_err!("Client {} not found", client_id))?
            .player
            .ok_or_else(|| format_err!("Client {} has no player", client_id))?;

        self.player_join_game(&player_id, game_id)
    }

    pub fn player_join_game(
        &mut self,
        player_id: &PlayerId,
        game_id: Option<GameId>,
    ) -> Result<GameId, Error> {
        self.player_part_game(player_id)
            .context("Failed to part current game")?;

        let game_id = match game_id {
            Some(game_id) => match self.games.contains_key(&game_id) {
                true => game_id,
                false => self.new_game(),
            },
            None => self.new_game(),
        };

        let game = self
            .games
            .get_mut(&game_id)
            .ok_or_else(|| format_err!("Game {} not found", game_id))?;

        if game.is_full() {
            return Err(format_err!("Game {} is full", game_id));
        }

        let player = self
            .players
            .get_mut(&player_id)
            .ok_or_else(|| format_err!("Player {} not found", player_id))?;

        player.game = Some(game_id);
        game.players.insert(player_id.clone());

        self.message_clients_in_game(
            &game_id,
            &MessageOut::GamePlayers(
                self.games
                    .get(&game_id)
                    .ok_or_else(|| format_err!("Game {} not found", game_id))?
                    .players
                    .iter()
                    .filter_map(|player_id| self.players.get(player_id))
                    .map(|player| NetworkPlayer::from(player))
                    .collect(),
            ),
        )
        .unwrap_or_else(|error| {
            error!(
                "Failed to send GamePlayers message: {}",
                get_error_chain(error)
            )
        });

        Ok(game_id)
    }

    pub fn client_start_game(&mut self, client_id: &ClientId) -> Result<DateTime<Utc>, Error> {
        let player_id = self
            .clients
            .get(&client_id)
            .ok_or_else(|| format_err!("Client {} not found", client_id))?
            .player
            .ok_or_else(|| format_err!("Client {} has no player", client_id))?;

        self.player_start_game(&player_id)
    }

    pub fn player_start_game(&mut self, player_id: &PlayerId) -> Result<DateTime<Utc>, Error> {
        let game_id = self
            .players
            .get(&player_id)
            .ok_or_else(|| format_err!("Player {} not found", player_id))?
            .game
            .ok_or_else(|| format_err!("Player {} not in a game", player_id))?;

        let game = self
            .games
            .get_mut(&game_id)
            .ok_or_else(|| format_err!("Game {} not found", game_id))?;

        let start_at = Utc::now() + OldDuration::seconds(GAME_START_TIMER_SECONDS);
        game.started = Some(start_at);

        Ok(start_at)
    }

    pub fn client_part_game(&mut self, client_id: &ClientId) -> Result<bool, Error> {
        let player_id = self
            .clients
            .get(&client_id)
            .ok_or_else(|| format_err!("Client {} not found", client_id))?
            .player
            .ok_or_else(|| format_err!("Client {} has no player", client_id))?;

        self.player_part_game(&player_id)
    }

    pub fn player_part_game(&mut self, player_id: &PlayerId) -> Result<bool, Error> {
        let player = self
            .players
            .get_mut(player_id)
            .ok_or_else(|| format_err!("Player {} not found", player_id))?;

        let current_game_id = match player.game {
            Some(game_id) => game_id,
            None => return Ok(false),
        };

        let current_game = self
            .games
            .get_mut(&current_game_id)
            .ok_or_else(|| format_err!("Game {} not found", current_game_id))?;

        player.game = None;
        current_game.players.remove(&player_id);

        // when player is removed, also remove their game (if it is now empty)
        if current_game.is_empty() {
            self.remove_game(current_game_id)
                .context("Failed to remove empty game")?;
        } else {
            self.message_clients_in_game(
                &current_game_id,
                &MessageOut::GamePlayers(
                    self.games
                        .get(&current_game_id)
                        .ok_or_else(|| format_err!("Game {} not found", current_game_id))?
                        .players
                        .iter()
                        .filter_map(|player_id| self.players.get(player_id))
                        .map(|player| NetworkPlayer::from(player))
                        .collect(),
                ),
            )
            .unwrap_or_else(|error| {
                error!(
                    "Failed to send GamePlayers message: {}",
                    get_error_chain(error)
                )
            });
        }

        Ok(true)
    }
}

impl Server {
    pub fn handle_message(
        &mut self,
        client_id: ClientId,
        payload: MessageInPayload,
    ) -> Result<(), Error> {
        match payload {
            MessageInPayload::Connection(message) => match message {
                ConnectionMessage::Connect(ip_address, address) => {
                    info!("Client connected: {}", client_id);
                    self.new_client(client_id, ip_address, address);
                    self.message_client(&client_id, &MessageOut::TotalGames(self.games.len()))
                        .context("Failed to send TotalGames")?;
                }
                ConnectionMessage::Disconnect => {
                    info!("Client disconnected: {}", client_id);
                    self.remove_client(client_id)
                        .context("Failed to remove client")?;
                }
            },
            MessageInPayload::Matchmaking(message) => match message {
                MatchmakingMessage::ConfigurePlayer { name, color } => {
                    let player_id = self
                        .configure_player(client_id, name, color)
                        .context("Failed to configure player")?;

                    info!("Client {} configured player {}", client_id, player_id);
                    self.message_client(&client_id, &MessageOut::PlayerId(player_id))
                        .context("Failed to send player id")?;
                }

                MatchmakingMessage::JoinGame(game_id) => {
                    let game_id = self
                        .client_join_game(&client_id, game_id)
                        .context("Failed to join game")?;

                    info!("Client {} joined game {}", client_id, game_id);
                    self.message_all_clients(&MessageOut::TotalGames(self.games.len()));
                    self.message_client(&client_id, &MessageOut::JoinedGame(game_id))
                        .context("Failed to send game id")?;
                }

                MatchmakingMessage::PartGame => {
                    let parted = self
                        .client_part_game(&client_id)
                        .context("Failed to part game")?;
                    if !parted {
                        return Err(format_err!("Client {} not in a game", client_id));
                    }

                    info!("Client {} parted game", client_id);
                    self.message_all_clients(&MessageOut::TotalGames(self.games.len()));
                    self.message_client(&client_id, &MessageOut::PartedGame)
                        .context("Failed to send PartedGame message")?;
                }
            },
            MessageInPayload::GameInput(message) => match message {
                GameInputMessage::StartGame => {
                    let start_at = self
                        .client_start_game(&client_id)
                        .context("Failed to start game")?;

                    info!("Client {} started game", client_id);

                    self.message_clients_in_game_from_client(
                        &client_id,
                        &MessageOut::GameStarting(start_at),
                    )
                    .context("Failed to send GameStarting message(s)")?;
                }
                GameInputMessage::Turn(_direction) => unimplemented!(),
            },
        }
        Ok(())
    }
}

impl Server {
    pub fn process_messages(&mut self) {
        // take ownership of queued messages
        let mut process_messages_queue = Vec::with_capacity(self.message_queue.len());
        process_messages_queue.append(&mut self.message_queue);

        // process each message
        process_messages_queue.drain(..).for_each(|message| {
            self.handle_message(message.client_id, message.payload)
                .unwrap_or_else(|error| {
                    error!("Failed processing MessageIn: {}", get_error_chain(error))
                });
        });
    }

    pub fn update(&mut self) -> Vec<(GameId, ArenaUpdates)> {
        let now = Utc::now();

        self.games
            .iter_mut()
            .filter(|(_, game)| game.started.map_or(false, |started| now >= started))
            .map(|(game_id, game)| (game_id.clone(), game.arena.update()))
            .collect()
    }

    pub fn send_updates(&mut self, updates: Vec<(GameId, ArenaUpdates)>) {
        for (game_id, game_updates) in updates {
            self.message_clients_in_game(&game_id, &MessageOut::PatchGameState(game_updates))
                .unwrap_or_else(|error| {
                    error!(
                        "Failed to send PatchGameState message: {}",
                        get_error_chain(error)
                    )
                });
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;

    fn started(&mut self, context: &mut Context<Self>) {
        context.run_interval(
            Duration::from_millis(UPDATE_RATE_MILLISECONDS),
            |server, _context| {
                server.process_messages();
                let updates = server.update();
                server.send_updates(updates);
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
