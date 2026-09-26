use futures::sink::{Sink, SinkExt};
use futures::stream::{Stream, StreamExt};
use log::{debug, error, trace, warn};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore, mpsc};
use tokio::time::{self, Instant};
use warp::http::StatusCode;
use warp::reply::{Reply, Response};
use warp::ws::{Message, WebSocket, Ws};

use crate::server::{ClientId, MessageIn, MessageOut};
use crate::truncate_for_log;

const PING_RATE_SECONDS: u64 = 15;
/// How long a client can go without sending anything, pongs included, before
/// it is disconnected. Clients answer the pings, so this is only reached
/// when the connection has gone.
const IDLE_TIMEOUT_SECONDS: u64 = 45;
/// How long a message can take to be written to the socket before the
/// client is disconnected.
const WRITE_TIMEOUT_SECONDS: u64 = 10;
/// Largest message or frame a client can send, in bytes.
const MAX_MESSAGE_BYTES: usize = 4 * 1024;
/// How many messages a client can send at once, and how many a second after
/// that. Messages over the limit are dropped.
const MESSAGE_BURST: f64 = 40.0;
const MESSAGES_PER_SECOND: f64 = 20.0;

pub fn websocket(
    ws: Ws,
    ip_address: Option<SocketAddr>,
    server_tx: Sender<MessageIn>,
    connections: Arc<Semaphore>,
) -> Response {
    let Ok(permit) = connections.try_acquire_owned() else {
        warn!("Refusing websocket connection: too many clients are connected");
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    let ip_address = ip_address.map(|ip_address| format!("{}", ip_address));
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .max_frame_size(MAX_MESSAGE_BYTES)
        .on_upgrade(|websocket| handle_websocket(websocket, ip_address, server_tx, permit))
        .into_response()
}

/// Handles one client's connection. `_permit` is its place among the
/// connections allowed at once, given up when the connection ends.
async fn handle_websocket(
    websocket: WebSocket,
    ip_address: Option<String>,
    server_tx: Sender<MessageIn>,
    _permit: OwnedSemaphorePermit,
) {
    let id = ClientId::default();

    let (messages_tx, messages_rx) = mpsc::channel::<MessageOut>(100);
    let (ws_tx, ws_rx) = websocket.split();
    let ws_tx = Arc::new(Mutex::new(ws_tx));

    // send new client
    if let Err(error) = server_tx
        .send(MessageIn::connect(id, ip_address, messages_tx))
        .await
    {
        error!("Failed to send new client to server: {}", error);
        return;
    }

    let mut in_task = tokio::spawn(handle_in(id, ws_rx, server_tx.clone()));
    let mut out_task = tokio::spawn(handle_out(messages_rx, ws_tx.clone()));
    let mut ping_task = tokio::spawn(handle_ping(ws_tx.clone()));

    if let Err(error) = select! {
        out = &mut in_task => out,
        out = &mut out_task => out,
        out = &mut ping_task => out,
    } {
        error!("Failure occurred while handling websocket: {}", error);
    }

    // Stop the other tasks and close the socket now. Otherwise they keep it
    // open, and the reply to a client's close frame isn't sent until the next
    // ping, leaving the client waiting.
    in_task.abort();
    out_task.abort();
    ping_task.abort();
    let close = async { ws_tx.lock().await.close().await };
    match time::timeout(Duration::from_secs(WRITE_TIMEOUT_SECONDS), close).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => trace!("Failed to close websocket: {:?}", error),
        Err(_) => trace!("Timed out closing websocket"),
    }

    // send client disconnect
    if let Err(error) = server_tx.send(MessageIn::disconnect(id)).await {
        error!("Failed to send client disconnect to server: {}", error);
    }
}

async fn handle_in(
    id: ClientId,
    mut rx: impl Stream<Item = Result<Message, warp::Error>> + Unpin,
    tx: Sender<MessageIn>,
) {
    debug!("Websocket handler (in) created");
    let mut rate_limit = RateLimit::new(Instant::now());
    loop {
        let message =
            match time::timeout(Duration::from_secs(IDLE_TIMEOUT_SECONDS), rx.next()).await {
                Ok(Some(message)) => message,
                Ok(None) => break,
                Err(_) => {
                    debug!("Client {} sent nothing for too long", id);
                    break;
                }
            };
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
                trace!("Non-text message received");
                continue;
            }
        };

        if !rate_limit.allow(Instant::now()) {
            trace!("Dropping message from client {} over the rate limit", id);
            continue;
        }

        trace!("Text message received: {}", truncate_for_log(text));
        let message = match MessageIn::from_json(id, text) {
            Ok(message) => message,
            Err(error) => {
                warn!(
                    "Failed to parse incoming message ({}): {}",
                    truncate_for_log(text),
                    error
                );
                continue;
            }
        };

        tx.send(message)
            .await
            .unwrap_or_else(|error| error!("Failed to send incoming message to server: {}", error))
    }
    debug!("Websocket handler (in) closed");
}

async fn handle_out(mut rx: Receiver<MessageOut>, tx: Arc<Mutex<impl Sink<Message> + Unpin>>) {
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

        if !write(&tx, Message::text(text)).await {
            error!("Failed to send outgoing message");
            break;
        }
    }
    debug!("Websocket handler (out) closed");
}

async fn handle_ping(tx: Arc<Mutex<impl Sink<Message> + Unpin>>) {
    debug!("Websocket handler (ping) created");
    let mut interval = time::interval(Duration::from_secs(PING_RATE_SECONDS));
    loop {
        interval.tick().await;
        if !write(&tx, Message::ping(Vec::new())).await {
            error!("Failed to send outgoing ping");
            break;
        }
    }
    debug!("Websocket handler (ping) closed");
}

/// Writes a message to the socket, giving up after `WRITE_TIMEOUT_SECONDS`.
/// Returns whether it was written.
async fn write(tx: &Mutex<impl Sink<Message> + Unpin>, message: Message) -> bool {
    let write = async { tx.lock().await.send(message).await.is_ok() };
    time::timeout(Duration::from_secs(WRITE_TIMEOUT_SECONDS), write)
        .await
        .unwrap_or(false)
}

/// Limits how often a client's messages are handled, allowing short bursts.
struct RateLimit {
    allowance: f64,
    updated: Instant,
}

impl RateLimit {
    fn new(now: Instant) -> Self {
        Self {
            allowance: MESSAGE_BURST,
            updated: now,
        }
    }

    /// Whether a message that arrived at `now` is within the limit.
    fn allow(&mut self, now: Instant) -> bool {
        let elapsed = now.saturating_duration_since(self.updated).as_secs_f64();
        self.allowance = (self.allowance + elapsed * MESSAGES_PER_SECOND).min(MESSAGE_BURST);
        self.updated = now;
        if self.allowance < 1.0 {
            return false;
        }
        self.allowance -= 1.0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_over_the_rate_limit_are_refused() {
        let start = Instant::now();
        let mut rate_limit = RateLimit::new(start);

        let allowed = (0..100).filter(|_| rate_limit.allow(start)).count();
        assert_eq!(allowed, MESSAGE_BURST as usize);

        // The allowance comes back over time.
        let later = start + Duration::from_millis(500);
        let allowed = (0..100).filter(|_| rate_limit.allow(later)).count();
        assert_eq!(allowed, (MESSAGES_PER_SECOND * 0.5) as usize);
    }
}
