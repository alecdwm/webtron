// use failure::Error;
// use failure::ResultExt;
use crate::config::Config;
use crate::server::{ClientConnection, IncomingMessage, OutgoingMessage, ServerMessage};
use actix::{Actor, ActorContext, AsyncContext, Handler, Running, StreamHandler};
use actix_web::{fs, server, ws, App};
use log::{error, info};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub fn run(server_tx: mpsc::Sender<ServerMessage>, config: &Config) {
    server::new(move || {
        let server_tx = Arc::new(Mutex::new(server_tx.clone()));

        App::new()
            .resource("/ws", |r| {
                r.f(move |req| {
                    ws::start(
                        req,
                        Ws {
                            id: Uuid::new_v4(),
                            c2s_tx: None,
                            server_tx: server_tx
                                .lock()
                                .expect("Failed to lock server_tx for new client")
                                .clone(),
                        },
                    )
                })
            })
            .handler(
                "/",
                fs::StaticFiles::new("client")
                    .expect("Failed to serve client from filesystem")
                    .index_file("index.html"),
            )
    })
    .bind(config.bind_addr)
    .expect("Failed to bind to socket")
    .run();
}

#[derive(Debug)]
struct Ws {
    id: Uuid,
    c2s_tx: Option<mpsc::Sender<IncomingMessage>>,
    server_tx: mpsc::Sender<ServerMessage>,
}

impl Actor for Ws {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let (c2s_tx, c2s_rx) = mpsc::channel();

        self.c2s_tx = Some(c2s_tx.clone());

        self.server_tx
            .send(ServerMessage::ClientConnected(ClientConnection {
                id: self.id,
                tx: ctx.address().recipient(),
                rx: c2s_rx,
            }))
            .expect("Failed to send new client connection to webtron server");
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        self.server_tx
            .send(ServerMessage::ClientDisconnected(self.id))
            .expect("Failed to send client disconnect to webtron server");

        Running::Stop
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for Ws {
    fn handle(&mut self, message: ws::Message, ctx: &mut Self::Context) {
        dbg!(&message);
        match message {
            ws::Message::Text(text) => {
                let message = serde_json::from_str(&text).expect("Failed to parse IncomingMessage");
                match self.c2s_tx.clone() {
                    Some(c2s_tx) => c2s_tx
                        .send(message)
                        .expect("Failed to send IncomingMessage"),
                    None => panic!("No c2s_tx"),
                }
            }
            ws::Message::Binary(_) => (),
            ws::Message::Ping(message) => ctx.pong(&message),
            ws::Message::Pong(_) => (),
            ws::Message::Close(_) => ctx.stop(),
        }
    }
}

impl Handler<OutgoingMessage> for Ws {
    type Result = ();

    fn handle(&mut self, message: OutgoingMessage, ctx: &mut Self::Context) {
        let text = serde_json::to_string(&message).expect("Unable to serialize OutgoingMessage");
        ctx.text(text);
    }
}
