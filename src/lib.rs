///
/// Useful macros (for internal use).
///
pub mod macros;

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
use server::{Leaderboard, Ntfy, Server as WebtronServer};

///
/// Starts up webtron server.
///
pub async fn start() -> Result<(), Error> {
    let config = Arc::new(Config::new());
    let leaderboard = Leaderboard::open(&config.leaderboard_path)?;
    let ntfy = Ntfy::new(config.ntfy_url.as_deref(), config.public_url.as_deref())?;

    let (server_tx, server_rx) = mpsc::channel(100);
    let server = tokio::spawn(WebtronServer::new(server_rx, leaderboard, ntfy).start());
    // The web server runs on this task rather than a spawned one: the future
    // returned by warp's server isn't provably `Send`, so it can't be spawned.
    let web = async {
        web::start(server_tx, config).await;
        Ok(())
    };

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

/// Longest client-supplied text written to the log, in characters.
const MAX_LOGGED_CHARACTERS: usize = 200;

///
/// Returns `text` cut to at most `MAX_LOGGED_CHARACTERS` characters, for
/// writing text that came from a client to the log.
///
pub fn truncate_for_log(text: &str) -> String {
    match text.char_indices().nth(MAX_LOGGED_CHARACTERS) {
        Some((end, _)) => format!("{}... ({} bytes)", &text[..end], text.len()),
        None => text.to_string(),
    }
}
