use crate::config::Config;
use crate::server::{ClientConnection, IncomingMessage, OutgoingMessage, ServerMessage};
use actix::{Actor, ActorContext, AsyncContext, Handler, Running, StreamHandler};
use actix_files::Files;
use actix_web::{error, web, App, HttpServer};
use actix_web_actors::ws;
use failure::{Error, ResultExt};
use log::{error, trace, warn};
use std::sync::{mpsc, Arc, Mutex};
use uuid::Uuid;

pub fn run(server_tx: mpsc::Sender<ServerMessage>, config: &Config) -> Result<(), Error> {
    HttpServer::new(move || {
        let server_tx = Arc::new(Mutex::new(server_tx.clone()));

        App::new()
            //
            // websocket handler
            //
            .service(web::resource("/ws").route(web::get().to(
                move |request, stream: web::Payload| {
                    let id = Uuid::new_v4();
                    let server_tx = server_tx
                        .lock()
                        .map_err(|error| {
                            error!("Failed to lock server_tx for new client: {}", error);
                            error::ErrorInternalServerError("")
                        })?
                        .clone();

                    ws::start(
                        WsClient {
                            id,
                            c2s_tx: None,
                            server_tx,
                        },
                        &request,
                        stream,
                    )
                },
            )))
            //
            // fs handler
            //
            .service(Files::new("/", "client").index_file("index.html"))
    })
    .bind(config.bind_addr)
    .context("Failed to bind to socket")?
    .run()
    .context("Failed to start HttpServer")?;

    Ok(())
}

#[derive(Debug)]
struct WsClient {
    id: Uuid,
    c2s_tx: Option<mpsc::Sender<IncomingMessage>>,
    server_tx: mpsc::Sender<ServerMessage>,
}

impl Actor for WsClient {
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
            .unwrap_or_else(|error| {
                error!(
                    "Failed to send new client connection to webtron server: {}",
                    error
                );
            });
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        self.server_tx
            .send(ServerMessage::ClientDisconnected(self.id))
            .unwrap_or_else(|error| {
                error!(
                    "Failed to send client disconnect to webtron server: {}",
                    error
                );
            });

        Running::Stop
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WsClient {
    fn handle(&mut self, message: ws::Message, ctx: &mut Self::Context) {
        match message {
            ws::Message::Text(text) => {
                trace!("Text message received: {}", text);

                let message = match serde_json::from_str(&text) {
                    Ok(val) => val,
                    Err(error) => {
                        warn!("Failed to parse incoming message ({}): {}", text, error);
                        return;
                    }
                };

                match self.c2s_tx.clone() {
                    Some(c2s_tx) => c2s_tx.send(message).unwrap_or_else(|error| {
                        error!(
                            "Failed to send incoming message to game server ({}): {}",
                            text, error
                        );
                    }),
                    None => error!(
                        "WebsocketClient is missing c2s_tx for communication with game server"
                    ),
                }
            }
            ws::Message::Binary(binary) => trace!("Binary message received: {:?}", binary),
            ws::Message::Ping(message) => {
                trace!("Ping received: {}", message);
                ctx.pong(&message)
            }
            ws::Message::Pong(message) => trace!("Pong received: {}", message),
            ws::Message::Close(message) => {
                trace!("Close received: {:?}", message);
                ctx.stop()
            }
            ws::Message::Nop => (),
        }
    }
}

impl Handler<OutgoingMessage> for WsClient {
    type Result = ();

    fn handle(&mut self, message: OutgoingMessage, ctx: &mut Self::Context) {
        let text = match serde_json::to_string(&message) {
            Ok(val) => val,
            Err(error) => {
                error!(
                    "Failed to serialize outgoing message ({:?}): {}",
                    message, error
                );
                return;
            }
        };
        ctx.text(text);
    }
}
