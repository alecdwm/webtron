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

//impl Server {
//    fn create_game(&mut self) -> Uuid {
//        let game = Game::default();
//        let game_id = game.id;
//        info!("Game created {:?}", game);
//        self.games.insert(game.id, game);
//        game_id
//    }

//    fn join_player_to_game(&mut self, client_id: Uuid, game_id: Uuid) -> Result<(), Error> {
//        let game = self.games.get_mut(&game_id).ok_or_else(|| {
//            format_err!(
//                "Player {} can't join game {}: game not found!",
//                client_id,
//                game_id
//            )
//        })?;

//        if game.is_full() {
//            return Err(format_err!(
//                "Player {} can't join game {}: game is full!",
//                client_id,
//                game_id
//            ));
//        }

//        let player = self.players.remove(&client_id).ok_or_else(|| {
//            format_err!(
//                "Player {} can't join game {}: player not found!",
//                client_id,
//                game_id
//            )
//        })?;

//        game.join_player(player);
//        info!("Player {} joined game {:?}", client_id, game);

//        self.clients
//            .get(&client_id)
//            .ok_or_else(|| {
//                format_err!(
//                    "Failed to send game id to client {}: client not found",
//                    client_id
//                )
//            })?
//            .address
//            .try_send(MessageOut::JoinedGame(game_id))
//            .with_context(|_| {
//                format_err!("Failed to send game id to client {}", client_id)
//            })?;

//        Ok(())
//    }

//}

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

                    self.clients
                        .get_mut(&client_id)
                        .ok_or_else(|| format_err!("Client not found: {}", client_id))?
                        .address
                        .try_send(MessageOut::PlayerId(player_id))
                        .with_context(|_| {
                            format_err!("Failed to send player id to client {}", client_id)
                        })?;
                }

                MatchmakingMessage::JoinGame(Some(game_id)) => {
                    let client = self.clients.get_mut(&client_id).ok_or_else(|| {
                        format_err!("Failed to join game: Client {} not found", client_id)
                    })?;

                    let player = match client.player {
                        Some(player_id) => {
                            Ok(self.players.get_mut(&player_id).ok_or_else(|| {
                                format_err!("Failed to join game: Player {} not found", player_id)
                            })?)
                        }
                        None => Err(format_err!(
                            "Failed to join game: Client {} has no player",
                            client_id
                        )),
                    }?;

                    // let player = client.player

                    // let player =
                    // if let Some(game_id) =
                    unimplemented!();
                    // self.part_player_from_games(client_id);

                    // let game_id = if !self.games.contains_key(&game_id) {
                    //     self.create_game()
                    // } else {
                    //     game_id
                    // };

                    // self.join_player_to_game(client_id, game_id)?;
                    // self.remove_empty_games();
                }
                MatchmakingMessage::JoinGame(None) => {
                    unimplemented!();
                    // self.part_player_from_games(client_id);

                    // let game_id = self.create_game();

                    // self.join_player_to_game(client_id, game_id)?;
                    // self.remove_empty_games();
                }

                MatchmakingMessage::PartGame => {
                    unimplemented!();
                    // self.part_player_from_games(client_id);
                    // self.remove_empty_games();
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
