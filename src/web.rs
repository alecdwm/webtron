mod embed;
mod websocket;

use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use warp::Filter;

use crate::config::Config;
use crate::server::MessageIn;
use embed::embed;
use websocket::websocket;

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
    // fs handler
    //
    let embed = embed();

    let routes = ws.or(embed);
    warp::serve(routes).bind(config.bind_address).await;
}
