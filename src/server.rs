// mod arena;
mod messages;
mod primitives;

use actix::{Actor, Context, Handler, Recipient};
use debug_stub_derive::DebugStub;
use failure::{format_err, Error, ResultExt};
use log::{error, info};
use std::collections::{HashMap, HashSet};

pub use messages::incoming::{ConnectionMessage, GameInputMessage, MatchmakingMessage};
pub use messages::{MessageIn, MessageOut};
pub use primitives::*;

use messages::MessageInPayload;

const MAX_PLAYERS_PER_GAME: usize = 8;

//
// Datastructure
//

#[derive(Debug, Default)]
pub struct Server {
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

        self.games.insert(
            game_id,
            Game {
                max_players: MAX_PLAYERS_PER_GAME,

                id: game_id,
                players: HashSet::new(),
            },
        );

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

            id: GameId::new_v4(),
            players: Default::default(),
        }
    }
}

impl Game {
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
        message: MessageOut,
    ) -> Result<(), Error> {
        self.clients
            .get_mut(client_id)
            .ok_or_else(|| format_err!("Client not found: {}", client_id))?
            .address
            .try_send(message)
            .with_context(|_| format_err!("Failed to message client {}", client_id))?;

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

        Ok(game_id)
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
                    self.message_client(&client_id, MessageOut::PlayerId(player_id))
                        .context("Failed to send player id")?;
                }

                MatchmakingMessage::JoinGame(game_id) => {
                    let game_id = self
                        .client_join_game(&client_id, game_id)
                        .context("Failed to join game")?;

                    info!("Client {} joined game {}", client_id, game_id);
                    self.message_client(&client_id, MessageOut::JoinedGame(game_id))
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
                    self.message_client(&client_id, MessageOut::PartedGame)
                        .context("Failed to send PartedGame message")?;
                }
            },
            MessageInPayload::GameInput(message) => match message {
                GameInputMessage::StartGame => unimplemented!(),
                GameInputMessage::Turn(_direction) => unimplemented!(),
            },
        }
        Ok(())
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<MessageIn> for Server {
    type Result = ();

    fn handle(&mut self, message: MessageIn, _context: &mut Context<Self>) {
        self.handle_message(message.client_id, message.payload)
            .unwrap_or_else(|error| error!("Failed processing MessageIn: {}", error));
    }
}
