use debug_stub_derive::DebugStub;
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::time;
use uuid::Uuid;

pub struct WebtronServer {
    _games: Vec<Game>,

    client_connections: HashMap<Uuid, ClientConnection>,
    server_rx: mpsc::Receiver<ServerMessage>,
}

impl WebtronServer {
    pub fn new() -> (WebtronServer, mpsc::Sender<ServerMessage>) {
        let (server_tx, server_rx) = mpsc::channel();

        let server = WebtronServer {
            _games: Vec::new(),

            client_connections: HashMap::new(),
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
                        self.client_connections.insert(client.id, client);
                    }
                    ClientDisconnected(id) => {
                        info!("Client disconnected: {}", id);
                        self.client_connections.remove(&id);
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
                                    games: vec!["One".to_owned(), "Two".to_owned()],
                                })
                                .unwrap_or_else(|error| {
                                    error!(
                                        "Failed to send games list to client {}: {}",
                                        client.id, error
                                    );
                                });
                        }
                        JoinGame(_game) => unimplemented!(),

                        ConfigurePlayer(_player) => unimplemented!(),
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
    GamesList { games: Vec<String> },
    PlayersList { players: Vec<Player> },

    PlayerSpawned { player: Player, x: f64, y: f64 },
    // GameState(GameState),
    PlayerDeath(Player),
}

#[derive(Debug, Deserialize)]
pub enum IncomingMessage {
    ListGames,
    JoinGame(String),

    ConfigurePlayer(Player),
    Spawn,
    Turn(TurnDirection),
}

#[derive(Debug)]
struct Game {
    players: Vec<Player>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub enum TurnDirection {
    Left,
    Right,
}
