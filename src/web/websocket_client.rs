use crate::server::{MessageIn, MessageOut, Server as WebtronServer};
use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Running, StreamHandler};
use actix_web_actors::ws::{self as websocket, WebsocketContext};
use log::{error, trace, warn};
use uuid::Uuid;

#[derive(Debug)]
pub struct WebsocketClient {
    id: Uuid,
    ip_address: Option<String>,
    server_address: Addr<WebtronServer>,
}

impl WebsocketClient {
    pub fn new(ip_address: Option<String>, server_address: Addr<WebtronServer>) -> Self {
        let id = Uuid::new_v4();

        WebsocketClient {
            id,
            ip_address,
            server_address,
        }
    }
}

impl Actor for WebsocketClient {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        self.server_address
            // TODO: send instead of try_send?
            .try_send(MessageIn::connect(
                self.id,
                self.ip_address.clone(),
                context.address().recipient(),
            ))
            .unwrap_or_else(|error| error!("Failed to send connect to webtron server: {}", error));
    }

    fn stopping(&mut self, _context: &mut Self::Context) -> Running {
        self.server_address
            // TODO: send instead of try_send?
            .try_send(MessageIn::disconnect(self.id))
            .unwrap_or_else(|error| {
                error!("Failed to send disconnect to webtron server: {}", error)
            });

        Running::Stop
    }
}

impl StreamHandler<websocket::Message, websocket::ProtocolError> for WebsocketClient {
    fn handle(&mut self, message: websocket::Message, context: &mut Self::Context) {
        match message {
            websocket::Message::Text(text) => {
                trace!("Text message received: {}", text);

                let message = unwrap_or_return!(
                    MessageIn::from_json(self.id, &text),
                    |error| warn!("Failed to parse incoming message ({}): {}", text, error)
                );

                // TODO: send instead of try_send?
                self.server_address
                    .try_send(message)
                    .unwrap_or_else(|error| {
                        error!("Failed to send message to webtron server: {}", error)
                    });
            }

            websocket::Message::Close(message) => {
                trace!("Close received: {:?}", message);
                context.stop()
            }

            websocket::Message::Ping(message) => {
                trace!("Ping received: {}", message);
                context.pong(&message)
            }
            websocket::Message::Binary(binary) => trace!("Binary message received: {:?}", binary),
            websocket::Message::Pong(message) => trace!("Pong received: {}", message),
            websocket::Message::Nop => trace!("Nop message received"),
        }
    }
}

impl Handler<MessageOut> for WebsocketClient {
    type Result = ();

    fn handle(&mut self, message: MessageOut, context: &mut Self::Context) {
        let text = unwrap_or_return!(message.to_json(), |error| error!(
            "Failed to serialize outgoing message ({:?}): {}",
            message, error
        ));
        context.text(text);
    }
}
