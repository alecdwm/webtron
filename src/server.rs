mod client;
mod messages;

pub use messages::{MessageIn, MessageOut};

use crate::game::{Game, Player, PlayerJoinable};
use actix::{Actor, Context, Handler};
use client::Client;
use failure::{format_err, Error, ResultExt};
use log::{error, info};
use messages::incoming::{ConnectionMessage, GameInputMessage, MatchmakingMessage};
use messages::MessageInHandler;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct Server {
    games: HashMap<Uuid, Game>,
    clients: HashMap<Uuid, Client>,
    players: HashMap<Uuid, Player>,
}

impl Server {
    pub fn new() -> Self {
        Default::default()
    }

    fn create_game(&mut self) -> Uuid {
        let game = Game::new();
        let game_id = game.id;
        info!("Game created {:?}", game);
        self.games.insert(game.id, game);
        game_id
    }

    fn join_player_to_game(&mut self, client_id: Uuid, game_id: Uuid) -> Result<(), Error> {
        let game = self.games.get_mut(&game_id).ok_or_else(|| {
            format_err!(
                "Player {} can't join game {}: game not found!",
                client_id,
                game_id
            )
        })?;

        if game.is_full() {
            return Err(format_err!(
                "Player {} can't join game {}: game is full!",
                client_id,
                game_id
            ));
        }

        let player = self.players.remove(&client_id).ok_or_else(|| {
            format_err!(
                "Player {} can't join game {}: player not found!",
                client_id,
                game_id
            )
        })?;

        game.join_player(player);
        info!("Player {} joined game {:?}", client_id, game);

        self.clients
            .get(&client_id)
            .ok_or_else(|| {
                format_err!(
                    "Failed to send game id to client {}: client not found",
                    client_id
                )
            })?
            .address()
            .try_send(MessageOut::JoinedGame(game_id))
            .with_context(|error| {
                format_err!("Failed to send game id to client {}: {}", client_id, error)
            })?;

        Ok(())
    }

    fn part_player_from_games(&mut self, client_id: Uuid) {
        let players =
            self.games
                .values_mut()
                .filter_map(|game| match game.part_player(&client_id) {
                    Some(player) => {
                        info!("Player {} left game {:?}", client_id, game);
                        Some(player)
                    }
                    None => None,
                });

        for player in players {
            self.players.insert(client_id, player);
        }
    }

    fn remove_empty_games(&mut self) {
        self.games.retain(|_, game| !game.is_empty());
    }
}

impl MessageInHandler for Server {
    ///
    /// Handle client (dis)connections
    ///
    fn handle_connection_message(
        &mut self,
        client_id: Uuid,
        message: ConnectionMessage,
    ) -> Result<(), Error> {
        match message {
            ConnectionMessage::Connect(ip_address, address) => {
                info!("Client connected: {}", client_id);
                let client = Client::new(client_id, ip_address, address);
                self.clients.insert(client_id, client);
            }
            ConnectionMessage::Disconnect => {
                info!("Client disconnected: {}", client_id);
                self.part_player_from_games(client_id);
                self.remove_empty_games();
                self.players.remove(&client_id);
                self.clients.remove(&client_id);
            }
        }
        Ok(())
    }

    ///
    /// Handle matchmaking messages from clients
    ///
    fn handle_matchmaking_message(
        &mut self,
        client_id: Uuid,
        message: MatchmakingMessage,
    ) -> Result<(), Error> {
        match message {
            MatchmakingMessage::ConfigurePlayer(mut player) => {
                self.part_player_from_games(client_id);

                let client = self.clients.get_mut(&client_id).ok_or_else(|| {
                    format_err!("ConfigurePlayer: Client {} not found!", client_id)
                })?;
                player.set_id(*client.id());

                info!("Client {} configured player {:?}", client_id, player);
                self.players.insert(*player.id(), player);

                client
                    .address()
                    .try_send(MessageOut::PlayerId(*client.id()))
                    .with_context(|error| {
                        format_err!(
                            "Failed to send player id to client {}: {}",
                            client_id,
                            error
                        )
                    })?;
            }

            MatchmakingMessage::JoinGame(Some(game_id)) => {
                self.part_player_from_games(client_id);

                let game_id = if !self.games.contains_key(&game_id) {
                    self.create_game()
                } else {
                    game_id
                };

                self.join_player_to_game(client_id, game_id)?;
                self.remove_empty_games();
            }
            MatchmakingMessage::JoinGame(None) => {
                self.part_player_from_games(client_id);

                let game_id = self.create_game();

                self.join_player_to_game(client_id, game_id)?;
                self.remove_empty_games();
            }

            MatchmakingMessage::PartGame => {
                self.part_player_from_games(client_id);
                self.remove_empty_games();
            }
        }
        Ok(())
    }

    ///
    /// Handle game input messages from clients
    ///
    fn handle_game_input_message(
        &mut self,
        _client_id: Uuid,
        message: GameInputMessage,
    ) -> Result<(), Error> {
        match message {
            GameInputMessage::StartGame => unimplemented!(),
            GameInputMessage::Turn(_direction) => unimplemented!(),
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<MessageIn> for Server {
    type Result = ();

    fn handle(&mut self, message: MessageIn, _context: &mut Context<Self>) {
        message
            .handle_with(self)
            .unwrap_or_else(|error| error!("{}", error));
    }
}
