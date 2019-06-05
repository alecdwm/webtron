mod game;
mod lobby;

use debug_stub_derive::DebugStub;
use failure::{format_err, Error};
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::time;
use uuid::Uuid;

use game::Game;
use lobby::Lobby;

#[derive(Debug)]
pub struct Server {
    lobby: Lobby,

    client_connections: HashMap<Uuid, ClientConnection>,
    players: HashMap<Uuid, Player>,

    server_rx: mpsc::Receiver<ServerMessage>,
}

impl Server {
    pub fn new() -> (Server, mpsc::Sender<ServerMessage>) {
        let (server_tx, server_rx) = mpsc::channel();

        let server = Server {
            lobby: Lobby::new(),

            client_connections: HashMap::new(),
            players: HashMap::new(),

            server_rx,
        };

        (server, server_tx)
    }

    pub fn run(mut self) {
        loop {
            // handle (dis)connects
            for message in self.server_rx.try_iter() {
                use ServerMessage::*;
                match message {
                    ClientConnected(client) => {
                        info!("New client connected: {}", client.id);
                        let id = client.id;
                        self.client_connections.insert(id, client);
                        self.players.insert(id, Player::default());
                        self.lobby.players_mut().insert(id);
                    }
                    ClientDisconnected(id) => {
                        info!("Client disconnected: {}", id);
                        self.client_connections.remove(&id);
                        self.players.remove(&id);
                    }
                }
            }

            // handle events from clients
            for client in self.client_connections.values() {
                for message in client.rx.try_iter() {
                    use IncomingMessage::*;
                    match message {
                        ListGames => {
                            client
                                .tx
                                .try_send(OutgoingMessage::GamesList {
                                    games: self.lobby.games().values().cloned().collect(),
                                })
                                .unwrap_or_else(|error| {
                                    error!(
                                        "Failed to send games list to client {}: {}",
                                        client.id, error
                                    );
                                });
                        }
                        NewGame(name) => {
                            self.lobby.new_game(&name);
                        }

                        JoinGame(game_uuid) => {
                            self.lobby.move_player_to_game(game_uuid, client.id);
                            dbg!(&self);
                        }

                        ConfigurePlayer(player) => {
                            self.players.insert(client.id, player);
                        }

                        Spawn => unimplemented!(),
                        Turn(_direction) => unimplemented!(),
                    }
                }
            }

            thread::sleep(time::Duration::from_millis(300));
        }
    }

    pub fn run_in_new_thread(self) -> thread::JoinHandle<()> {
        thread::spawn(move || self.run())
    }
}

#[derive(Debug)]
pub enum ServerMessage {
    ClientConnected(ClientConnection),
    ClientDisconnected(Uuid),
}

#[derive(DebugStub)]
pub struct ClientConnection {
    pub id: Uuid,
    #[debug_stub = "Recipient"]
    pub tx: actix::Recipient<OutgoingMessage>,
    pub rx: mpsc::Receiver<IncomingMessage>,
}

#[derive(Debug, Serialize, actix::Message)]
pub enum OutgoingMessage {
    GamesList { games: Vec<Game> },
    PlayersList { players: Vec<Player> },

    PlayerSpawned { player: Player, x: f64, y: f64 },
    // GameState(GameState),
    PlayerDeath(Player),
}

#[derive(Debug, Deserialize)]
pub enum IncomingMessage {
    ListGames,
    NewGame(String),
    JoinGame(Uuid),

    ConfigurePlayer(Player),
    Spawn,
    Turn(TurnDirection),
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Player {
    name: String,
    color: PlayerColor,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerColor {
    Blue,
    Green,
    Orange,
    Purple,
    Red,
    White,
}

impl Default for PlayerColor {
    fn default() -> Self {
        PlayerColor::Orange
    }
}

#[derive(Debug, Deserialize)]
pub enum TurnDirection {
    Left,
    Right,
}
