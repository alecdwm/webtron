///
/// Handles server configuration.
///
pub mod config;

///
/// Handles serverside game logic.
///
pub mod server;

///
/// Serves the client and handles websocket connections.
///
pub mod web;

use anyhow::{Context, Error};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::try_join;

use config::Config;
use server::Server as WebtronServer;

///
/// Starts up webtron server.
///
pub async fn start() -> Result<(), Error> {
    let config = Arc::new(Config::new());

    let (server_tx, server_rx) = mpsc::channel(100);
    let server = tokio::spawn(WebtronServer::new(server_rx).start());
    let web = tokio::spawn(web::start(server_tx, config));

    try_join!(server, web).context("Failure occurred in task")?;

    Ok(())
}

///
/// Returns a string representing the source hierarchy of an error.
/// Format:
///
///   `"$error: $source: $source_2: $source_3: etc"`
///
pub fn get_error_chain(error: Error) -> String {
    error
        .chain()
        .map(|f| format!("{}", f))
        .collect::<Vec<_>>()
        .join(": ")
}
