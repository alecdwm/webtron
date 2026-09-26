mod embed;
mod errors;
mod proxy;
mod websocket;

use log::warn;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::sync::mpsc::Sender;
use warp::{Filter, Reply};

use crate::config::Config;
use crate::server::MessageIn;
use embed::embed;
use errors::handle_rejection;
use proxy::proxy;
use websocket::websocket;

#[cfg(debug_assertions)]
const DEBUG: bool = true;
#[cfg(not(debug_assertions))]
const DEBUG: bool = false;

/// Most websocket connections open at once.
const MAX_CONNECTIONS: usize = 256;

pub async fn start(server_tx: Sender<MessageIn>, config: Arc<Config>) {
    let server_tx = warp::any().map(move || server_tx.clone());
    let connections = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    let connections = warp::any().map(move || connections.clone());

    //
    // websocket handler
    //
    let ws = warp::path("ws")
        .and(warp::path::end())
        .and(warp::ws())
        .and(warp::addr::remote())
        .and(server_tx)
        .and(connections)
        .map(websocket);

    //
    // proxy handler
    //
    let proxy = proxy("http://localhost:3001");

    //
    // fs handler
    //
    let embed = embed();

    //
    // use proxy in development
    // use fs (binary-embedded) in release
    //
    if DEBUG && !config.bind_address.ip().is_loopback() {
        warn!(
            "Serving the development proxy to the client on {}, which isn't a loopback address",
            config.bind_address
        );
    }
    let frontend = if DEBUG {
        proxy.map(|reply| Box::new(reply) as Box<dyn Reply>).boxed()
    } else {
        embed.map(|reply| Box::new(reply) as Box<dyn Reply>).boxed()
    };

    let routes = ws.or(frontend).recover(handle_rejection);
    warp::serve(routes).run(config.bind_address).await;
}
