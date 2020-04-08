use futures::sink::{Sink, SinkExt};
use futures::stream::{Stream, StreamExt};
use log::{debug, error, trace, warn};
use std::net::SocketAddr;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use warp::reply::Reply;
use warp::ws::{Message, WebSocket, Ws};

use crate::server::{ClientId, MessageIn, MessageOut};

pub fn websocket(
    ws: Ws,
    ip_address: Option<SocketAddr>,
    server_tx: Sender<MessageIn>,
) -> impl Reply {
    let ip_address = ip_address.map(|ip_address| format!("{}", ip_address));
    ws.on_upgrade(|websocket| handle_websocket(websocket, ip_address, server_tx))
}

async fn handle_websocket(
    websocket: WebSocket,
    ip_address: Option<String>,
    mut server_tx: Sender<MessageIn>,
) {
    let id = ClientId::new_v4();

    let (messages_tx, messages_rx) = mpsc::channel::<MessageOut>(100);
    let (ws_tx, ws_rx) = websocket.split();

    // send new client
    if let Err(error) = server_tx
        .send(MessageIn::connect(id, ip_address, messages_tx))
        .await
    {
        error!("Failed to send new client to server: {}", error);
        return;
    }

    let in_task = tokio::spawn(handle_in(id, ws_rx, server_tx.clone()));
    let out_task = tokio::spawn(handle_out(messages_rx, ws_tx));

    if let Err(error) = select! {
        out = in_task => out,
        out = out_task => out,
    } {
        error!("Failure occurred while handling websocket: {}", error);
    }

    // send client disconnect
    if let Err(error) = server_tx.send(MessageIn::disconnect(id)).await {
        error!("Failed to send client disconnect to server: {}", error);
    }
}

async fn handle_in(
    id: ClientId,
    mut rx: impl Stream<Item = Result<Message, warp::Error>> + Unpin,
    mut tx: Sender<MessageIn>,
) {
    debug!("Websocket handler (in) created");
    while let Some(message) = rx.next().await {
        let message = match message {
            Ok(message) => message,
            Err(error) => {
                error!("Error occurred in incoming message: {}", error);
                break;
            }
        };

        if message.is_close() {
            trace!("Close received: {:?}", message);
            break;
        }

        let text = match message.to_str() {
            Ok(text) => text,
            Err(()) => {
                trace!("Non-text message received: {:?}", message);
                continue;
            }
        };

        trace!("Text message received: {}", text);
        let message = match MessageIn::from_json(id, text) {
            Ok(message) => message,
            Err(error) => {
                warn!("Failed to parse incoming message ({}): {}", text, error);
                continue;
            }
        };

        tx.send(message)
            .await
            .unwrap_or_else(|error| error!("Failed to send incoming message to server: {}", error))
    }
    debug!("Websocket handler (in) closed");
}

async fn handle_out(mut rx: Receiver<MessageOut>, mut tx: impl Sink<Message> + Unpin) {
    debug!("Websocket handler (out) created");
    while let Some(message) = rx.recv().await {
        let text = match message.to_json() {
            Ok(text) => text,
            Err(error) => {
                error!(
                    "Failed to serialize outgoing message: ({:?}): {}",
                    message, error
                );
                continue;
            }
        };

        if tx.send(Message::text(text)).await.is_err() {
            error!("Failed to send outgoing message")
        }
    }
    debug!("Websocket handler (out) closed");
}
