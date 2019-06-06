use crate::server::{MessageIn, MessageOut, Server as WebtronServer};
use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Running, StreamHandler};
use actix_web_actors::ws::{self as websocket, WebsocketContext};
use log::{error, trace, warn};
use uuid::Uuid;

#[derive(Debug)]
pub struct WebsocketClient {
    id: Uuid,
    server_addr: Addr<WebtronServer>,
}

impl WebsocketClient {
    pub fn new(server_addr: Addr<WebtronServer>) -> Self {
        let id = Uuid::new_v4();

        WebsocketClient { id, server_addr }
    }
}

impl Actor for WebsocketClient {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.server_addr
            .try_send(MessageIn::connect(self.id, ctx.address().recipient())) // TODO: send instead of try_send?
            .unwrap_or_else(|error| error!("Failed to send connect to webtron server: {}", error));
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        self.server_addr
            .try_send(MessageIn::disconnect(self.id)) // TODO: send instead of try_send?
            .unwrap_or_else(|error| {
                error!("Failed to send disconnect to webtron server: {}", error)
            });

        Running::Stop
    }
}

impl StreamHandler<websocket::Message, websocket::ProtocolError> for WebsocketClient {
    fn handle(&mut self, message: websocket::Message, ctx: &mut Self::Context) {
        match message {
            websocket::Message::Text(text) => {
                trace!("Text message received: {}", text);

                let message = handle_err!(MessageIn::from_json(self.id, &text), |error| warn!(
                    "Failed to parse incoming message ({}): {}",
                    text, error
                ));

                // TODO: send instead of try_send?
                self.server_addr.try_send(message).unwrap_or_else(|error| {
                    error!("Failed to send message to webtron server: {}", error)
                });
            }
            websocket::Message::Close(message) => {
                trace!("Close received: {:?}", message);
                ctx.stop()
            }
            websocket::Message::Ping(message) => {
                trace!("Ping received: {}", message);
                ctx.pong(&message)
            }
            websocket::Message::Binary(binary) => trace!("Binary message received: {:?}", binary),
            websocket::Message::Pong(message) => trace!("Pong received: {}", message),
            websocket::Message::Nop => trace!("Nop message received"),
        }
    }
}

impl Handler<MessageOut> for WebsocketClient {
    type Result = ();

    fn handle(&mut self, message: MessageOut, ctx: &mut Self::Context) {
        let text = handle_err!(message.to_json(), |error| error!(
            "Failed to serialize outgoing message ({:?}): {}",
            message, error
        ));
        ctx.text(text);
    }
}
