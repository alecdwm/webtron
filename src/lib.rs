pub mod config;
// use failure::Error;
// use failure::ResultExt;

pub mod game_server {
    use crate::config::Config;
    use failure::Error;
    use futures::{Async, Future, IntoFuture, Poll, Stream};
    use serde_derive::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread;
    use std::time;
    use uuid::Uuid;

    pub struct WebtronServer {
        games: Vec<Game>,

        client_connections: HashMap<Uuid, ClientConnection>,
        server_rx: mpsc::Receiver<ServerMessage>,
    }
    impl WebtronServer {
        pub fn new() -> (WebtronServer, mpsc::Sender<ServerMessage>) {
            let (server_tx, server_rx) = mpsc::channel();
            (
                WebtronServer {
                    games: Vec::new(),

                    client_connections: HashMap::new(),
                    server_rx,
                },
                server_tx,
            )
        }

        pub fn run(mut self) {
            loop {
                for message in self.server_rx.try_iter() {
                    match message {
                        ServerMessage::ClientConnected(client) => {
                            // TODO: log: 'new client connected'
                            dbg!(&client);
                            self.client_connections.insert(client.id, client);
                        }
                        ServerMessage::ClientDisconnected(id) => {
                            // TODO: log: 'client disconnected'
                            self.client_connections.remove(&id);
                        }
                    }
                }

                for client in self.client_connections.iter() {
                    dbg!(&client);
                }
                // for message in self.rx.iter() {
                //     dbg!(&message);
                //     match message {
                //         WebtronServerMessage::NewClient(rx, tx) => {
                //             println!("New client!");

                //             // let (tx, rx) = websocket.split();
                //             // rx.forward(tx).map(|_| ()).map_err(|e| {
                //             //     eprintln!("websocket error: {:?}", e);
                //             // });
                //             // tx.
                //             // tx.send(Message::text("Hello!"));
                //         }
                //         _ => (),
                //     };
                // }
                thread::sleep(time::Duration::from_millis(300));
            }
        }

        pub fn run_in_thread(self) {
            thread::spawn(move || self.run());
        }
    }

    #[derive(Debug)]
    pub enum ServerMessage {
        ClientConnected(ClientConnection),
        ClientDisconnected(Uuid),
    }

    #[derive(Debug)]
    pub struct ClientConnection {
        pub id: Uuid,
        pub tx: mpsc::Sender<OutgoingMessage>,
        pub rx: mpsc::Receiver<IncomingMessage>,
    }

    #[derive(Debug, Serialize)]
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
        Turn(Direction),
    }

    #[derive(Debug, Deserialize)]
    pub enum Direction {
        Left,
        Right,
    }

    #[derive(Debug)]
    pub enum WebtronServerMessage {
        Test,
        NewClient(mpsc::Receiver<ClientMessage>, mpsc::Sender<ClientMessage>),
        RemovedClient(),
    }

    #[derive(Debug)]
    pub enum ClientMessageOut {
        Die,
    }

    pub enum ClientMessage {
        Spawn,
        Die,
        Close,
    }

    struct Game {
        players: Vec<Player>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Player {
        name: String,
        color: PlayerColor,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub enum PlayerColor {
        Blue,
        Green,
        Orange,
        Purple,
        Red,
        White,
    }
}

pub mod web {
    use crate::config::Config;
    use crate::game_server::{ClientConnection, IncomingMessage, OutgoingMessage, ServerMessage};
    use failure::Error;
    use futures::{Future, IntoFuture, Sink, Stream};
    use std::sync::{mpsc, Arc, Mutex};
    use uuid::Uuid;
    use warp::{filters::BoxedFilter, Filter, Reply};

    // implementation help:
    // https://github.com/seanmonstar/warp/blob/master/examples/websockets_chat.rs
    // https://github.com/seanmonstar/warp/blob/master/examples/todos.rs

    pub fn run(server_tx: mpsc::Sender<ServerMessage>, config: &Config) {
        // TODO: Consider using channels to send network events
        // to single-threaded WebtronServer instead of locking
        // access to WebtronServer from multiple threads
        //
        // See: https://doc.rust-lang.org/book/ch16-02-message-passing.html

        let routes = get_ws(server_tx).or(get_root());
        warp::serve(routes).run(config.bind_addr);
    }

    /// GET /
    fn get_root() -> BoxedFilter<(impl Reply,)> {
        warp::get2().and(warp::filters::fs::dir("client")).boxed()
    }

    /// GET /ws
    fn get_ws(server_tx: mpsc::Sender<ServerMessage>) -> BoxedFilter<(impl Reply,)> {
        let server_tx = Arc::new(Mutex::new(server_tx));

        warp::get2()
            .and(warp::path("ws"))
            .and(warp::filters::ws::ws2())
            .map(move |ws: warp::ws::Ws2| {
                let server_tx = server_tx
                    .lock()
                    .expect("Failed to lock server_tx for new client")
                    .clone();

                ws.on_upgrade(move |websocket| {
                    let client_id = Uuid::new_v4();
                    let (c2s_tx, c2s_rx) = mpsc::channel();
                    let (s2c_tx, s2c_rx) = mpsc::channel();

                    let (websocket_tx, websocket_rx) = websocket.split();

                    server_tx
                        .send(ServerMessage::ClientConnected(ClientConnection {
                            id: client_id,
                            tx: s2c_tx,
                            rx: c2s_rx,
                        }))
                        .expect("Failed to send new client connection to webtron server");

                    // websocket_rx.for_each(move |message| {
                    //     dbg!(&message);
                    // })
                    // s2c_rx
                    //     .forward(websocket_tx)
                    //     .join(websocket_rx.forward(c2s_tx));

                    websocket_rx
                        .for_each(move |message| {
                            let incoming_message = match &message.to_str() {
                                Ok(val) => match serde_json::from_str(val) {
                                    Ok(val) => val,
                                    Err(error) => {
                                        // TODO: Log 'error parsing incoming message'
                                        //        or 'ignoring incorrectly formatted incoming message'
                                        eprintln!("Error paring incoming message: {:?}", error);
                                        // return Err(error);
                                        return Ok(());
                                    }
                                },
                                Err(_) => {
                                    // TODO: Log 'ignoring incoming non-text message'
                                    return Ok(());
                                }
                            };
                            c2s_tx
                                .send(incoming_message)
                                .expect("Failed to send incoming message to webtron server");

                            Ok(())
                        })
                        .then(move |result| {
                            server_tx
                                .send(ServerMessage::ClientDisconnected(client_id))
                                .expect("Failed to send client disconnect to webtron server");
                            // TODO: handle disconnect
                            result
                        })
                        .map_err(move |error| {
                            // TODO: handle error
                            eprintln!("websocket error: {:?}", error);
                        })
                    // .join()
                    // tx.send("Hello!");

                    // server_tx.send(WebtronServerMessage::NewClient(websocket));

                    // Ok(()).into_future()
                    // rx.forward(tx).map(|_| ()).map_err(|e| {
                    //     eprintln!("websocket error: {:?}", e);
                    // })
                })
            })
            .boxed()
    }
}
