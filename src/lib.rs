///
/// Useful macros (for internal use).
///
#[macro_use]
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

use actix::{Actor, System};
use anyhow::{Context, Error};
use std::env;

use config::Config;
use server::Server as WebtronServer;

///
/// Sets up environment, creates event loop,
/// starts Actix actors and runs event loop.
///
pub fn start() -> Result<(), Error> {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "webtron=trace");
    }
    pretty_env_logger::init();

    let config = Config::new();

    let system = System::new("webtron");
    let server_address = WebtronServer::new().start();
    web::start(server_address, &config).context("Failed to start HttpServer")?;

    system
        .run()
        .context("Failed to start webtron actix system arbiter")?;

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
