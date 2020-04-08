mod embed;
mod errors;
mod proxy;
mod websocket;

use std::sync::Arc;
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

pub async fn start(server_tx: Sender<MessageIn>, config: Arc<Config>) {
    let server_tx = warp::any().map(move || server_tx.clone());

    //
    // websocket handler
    //
    let ws = warp::path("ws")
        .and(warp::path::end())
        .and(warp::ws())
        .and(warp::addr::remote())
        .and(server_tx)
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
    let frontend = if DEBUG {
        proxy.map(|reply| Box::new(reply) as Box<dyn Reply>).boxed()
    } else {
        embed.map(|reply| Box::new(reply) as Box<dyn Reply>).boxed()
    };

    let routes = ws.or(frontend).recover(handle_rejection);
    warp::serve(routes).bind(config.bind_address).await;
}
