mod client;
mod messages;

pub use messages::{MessageIn, MessageOut};

use self::client::Client;
use self::messages::{ClientMessageIn, InGameMessageIn, LobbyMessageIn, MessageInData};
use crate::game::{Game, Lobby, Player, PlayerJoinable};
use actix::{Actor, Context, Handler};
use failure::{format_err, Error, ResultExt};
use log::{error, info};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct Server {
    lobby: Lobby,
    games: HashMap<Uuid, Game>,
    clients: HashMap<Uuid, Client>,
    players: HashMap<Uuid, Player>,
}

impl Server {
    pub fn new() -> Self {
        Default::default()
    }

    fn create_game(&mut self, name: &str) -> Uuid {
        let game = Game::new(name);
        let game_id = game.id.clone();
        info!("Game created {:?}", game);
        self.games.insert(game.id, game);
        return game_id;
    }

    fn move_player_to_game(&mut self, client_id: Uuid, game_id: Uuid) -> Result<(), Error> {
        let game = self.games.get_mut(&game_id).ok_or_else(|| {
            format_err!(
                "Player {} can't join game {}: game not found!",
                client_id,
                game_id
            )
        })?;

        if game.max_players <= game.players.len() {
            return Err(format_err!(
                "Player {} can't join game {}: game is full!",
                client_id,
                game_id
            ));
        }

        if !self.lobby.remove_player(&client_id) {
            return Err(format_err!(
                "Player {} can't join game {}: player not in lobby!",
                client_id,
                game_id
            ));
        }

        game.add_player(&client_id);
        info!("Player {} joined game {:?}", client_id, game);

        Ok(())
    }

    fn move_player_to_lobby(&mut self, client_id: Uuid) -> Result<(), Error> {
        let game = self
            .games
            .values_mut()
            .find(|game| game.players().contains(&client_id))
            .ok_or_else(|| {
                format_err!(
                    "Player {} can't leave game: player not in any game!",
                    client_id
                )
            })?;

        game.remove_player(&client_id);
        self.lobby.add_player(&client_id);
        info!("Player {} left game {:?}", client_id, game);

        Ok(())
    }

    fn remove_empty_games(&mut self) {
        self.games.retain(|_, game| !game.players.is_empty());
    }

    fn handle_client_message(&mut self, client_id: Uuid, message: ClientMessageIn) {
        use ClientMessageIn::*;
        match message {
            //
            // Handle client (dis)connections
            //
            Connect(ip_address, addr) => {
                info!("Client connected: {}", client_id);
                let client = Client::new(client_id, ip_address, addr);
                self.clients.insert(client_id, client);
                self.lobby.add_player(&client_id);
            }
            Disconnect => {
                info!("Client disconnected: {}", client_id);
                self.games.values_mut().for_each(|game| {
                    game.remove_player(&client_id);
                });
                self.remove_empty_games();
                self.lobby.remove_player(&client_id);
                self.players.remove(&client_id);
                self.clients.remove(&client_id);
            }
        }
    }

    fn handle_lobby_message(
        &mut self,
        client_id: Uuid,
        message: LobbyMessageIn,
    ) -> Result<(), Error> {
        use LobbyMessageIn::*;
        match message {
            //
            // Handle lobby messages from clients
            //
            ConfigurePlayer(mut player) => {
                let client = self.clients.get_mut(&client_id).ok_or_else(|| {
                    format_err!("ConfigurePlayer: Client {} not found!", client_id)
                })?;
                player.set_id(*client.id());

                info!("Client {} configured player {:?}", client_id, player);
                self.players.insert(*player.id(), player);

                client
                    .addr()
                    .try_send(MessageOut::PlayerId(*client.id()))
                    .with_context(|error| {
                        format_err!(
                            "Failed to send player id to client {}: {}",
                            client_id,
                            error
                        )
                    })?;
            }

            FetchLobbyData => {
                let client = self.clients.get_mut(&client_id).ok_or_else(|| {
                    format_err!("FetchLobbyData: Client {} not found!", client_id)
                })?;

                client
                    .addr()
                    .try_send(MessageOut::LobbyData {
                        games: self.games.values().cloned().collect(),
                        players: self.players.values().cloned().collect(),
                    })
                    .with_context(|error| {
                        format_err!(
                            "Failed to send games list to client {}: {}",
                            client_id,
                            error
                        )
                    })?;
            }

            CreateGame(name) => {
                if !self.lobby.players().contains(&client_id) {
                    return Err(format_err!(
                        "Player {} can't create game {}: player not in lobby!",
                        client_id,
                        name
                    ));
                }
                let game_id = self.create_game(&name);
                self.move_player_to_game(client_id, game_id)?;
                self.remove_empty_games();
            }
            JoinGame(game_id) => {
                self.move_player_to_game(client_id, game_id)?;
                self.remove_empty_games();
            }
            LeaveGame => {
                self.move_player_to_lobby(client_id)?;
                self.remove_empty_games();
            }
        }
        Ok(())
    }

    fn handle_ingame_message(&mut self, _client_id: Uuid, message: InGameMessageIn) {
        use InGameMessageIn::*;
        match message {
            //
            // Handle ingame messages from clients
            //
            Spawn => unimplemented!(),
            Turn(_direction) => unimplemented!(),
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<MessageIn> for Server {
    type Result = ();

    fn handle(&mut self, message: MessageIn, _ctx: &mut Context<Self>) {
        let client_id = message.client_id;

        use MessageInData::*;
        match message.data {
            Client(message) => {
                self.handle_client_message(client_id, message);
                Ok(())
            }
            Lobby(message) => self.handle_lobby_message(client_id, message),
            InGame(message) => {
                self.handle_ingame_message(client_id, message);
                Ok(())
            }
        }
        .unwrap_or_else(|error| error!("{}", error));
    }
}
