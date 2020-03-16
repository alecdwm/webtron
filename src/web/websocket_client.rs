use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Running, StreamHandler};
use actix_web::{web, Error as ActixError, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use log::{error, trace, warn};
use uuid::Uuid;

use crate::server::{MessageIn, MessageOut, Server as WebtronServer};

pub async fn websocket_route(
    request: HttpRequest,
    stream: web::Payload,
    server_address: web::Data<Addr<WebtronServer>>,
) -> Result<HttpResponse, ActixError> {
    ws::start(
        WebsocketClient {
            id: Uuid::new_v4(),
            ip_address: request.connection_info().remote().map(str::to_owned),
            server_address: server_address.get_ref().clone(),
        },
        &request,
        stream,
    )
}

#[derive(Debug)]
struct WebsocketClient {
    id: Uuid,
    ip_address: Option<String>,
    server_address: Addr<WebtronServer>,
}

impl Actor for WebsocketClient {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        self.server_address
            .try_send(MessageIn::connect(
                self.id,
                self.ip_address.clone(),
                context.address().recipient(),
            ))
            .unwrap_or_else(|error| error!("Failed to send connect to webtron server: {}", error));
    }

    fn stopping(&mut self, _context: &mut Self::Context) -> Running {
        self.server_address
            .try_send(MessageIn::disconnect(self.id))
            .unwrap_or_else(|error| {
                error!("Failed to send disconnect to webtron server: {}", error)
            });

        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketClient {
    fn handle(
        &mut self,
        message: Result<ws::Message, ws::ProtocolError>,
        context: &mut Self::Context,
    ) {
        let message = unwrap_or_return!(message, |error| error!(
            "Failed to handle incoming message: {}",
            error
        ));

        match message {
            ws::Message::Text(text) => {
                trace!("Text message received: {}", text);

                let message = unwrap_or_return!(
                    MessageIn::from_json(self.id, &text),
                    |error| warn!("Failed to parse incoming message ({}): {}", text, error)
                );

                self.server_address
                    .try_send(message)
                    .unwrap_or_else(|error| {
                        error!("Failed to send message to webtron server: {}", error)
                    });
            }

            ws::Message::Close(message) => {
                trace!("Close received: {:?}", message);
                context.stop()
            }

            ws::Message::Ping(message) => {
                trace!("Ping received: {:?}", message);
                context.pong(&message)
            }
            ws::Message::Binary(binary) => trace!("Binary message received: {:?}", binary),
            ws::Message::Pong(message) => trace!("Pong received: {:?}", message),
            ws::Message::Continuation(message) => trace!("Continuation received: {:?}", message),
            ws::Message::Nop => trace!("Nop message received"),
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
