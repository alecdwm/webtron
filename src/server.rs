mod client;
mod messages;

pub use messages::{MessageIn, MessageOut};

use self::client::Client;
use self::messages::MessageInData;
use crate::game::{Game, Lobby, PlayerJoinable};
use actix::{Actor, Context, Handler};
use log::{error, info, warn};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct Server {
    lobby: Lobby,
    games: HashMap<Uuid, Game>,
    clients: HashMap<Uuid, Client>,
}

impl Server {
    pub fn new() -> Self {
        Default::default()
    }

    fn create_game(&mut self, name: &str) {
        let game = Game::new(name);
        info!("Game created {:?}", game);
        self.games.insert(game.id, game);
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<MessageIn> for Server {
    type Result = ();

    fn handle(&mut self, message: MessageIn, _ctx: &mut Context<Self>) {
        use MessageInData::*;

        let client_id = message.client_id;
        match message.data {
            //
            // Handle client (dis)connections
            //
            Connect(addr) => {
                info!("Client connected: {}", client_id);
                let client = Client::new(client_id, addr);
                self.clients.insert(client_id, client);
                self.lobby.add_player(&client_id);
            }
            Disconnect => {
                info!("Client disconnected: {}", client_id);
                self.games
                    .values_mut()
                    .for_each(|game| game.remove_player(&client_id));
                self.lobby.remove_player(&client_id);
                self.clients.remove(&client_id);
            }

            //
            // Handle client messages
            //
            ConfigurePlayer(player) => {
                let client = handle_none!(self.clients.get_mut(&client_id), {
                    error!("ConfigurePlayer: Client {} not found!", client_id);
                });
                info!("Client {} configured player {:?}", client_id, player);
                client.configure_player(player);
            }

            ListGames => {
                let client = handle_none!(self.clients.get_mut(&client_id), {
                    error!("ListGames: Client {} not found!", client_id);
                });

                handle_err!(
                    client.addr().try_send(MessageOut::GamesList {
                        games: self.games.values().cloned().collect(),
                    }),
                    error,
                    error!(
                        "Failed to send games list to client {}: {}",
                        client.id(),
                        error
                    )
                );
            }
            CreateGame(name) => self.create_game(&name),
            JoinGame(game_id) => {
                let game = handle_none!(self.games.get_mut(&game_id), {
                    error!(
                        "Player {} can't join game {}: game not found!",
                        client_id, game_id
                    );
                });

                if !self.lobby.players().contains(&client_id) {
                    warn!(
                        "Player {} can't join game {}: player not in lobby!",
                        client_id, game_id
                    );
                    return;
                }

                self.lobby.remove_player(&client_id);
                game.add_player(&client_id);
                info!("Player {} joined game {:?}", client_id, game);
            }
            LeaveGame => {
                let game = handle_none!(
                    self.games
                        .values_mut()
                        .find(|game| game.players().contains(&client_id)),
                    warn!(
                        "Player {} can't leave game: player not in any game!",
                        client_id
                    )
                );

                game.remove_player(&client_id);
                self.lobby.add_player(&client_id);
                info!("Player {} left game {:?}", client_id, game);
            }

            Spawn => unimplemented!(),
            Turn(_direction) => unimplemented!(),
        }
    }
}
