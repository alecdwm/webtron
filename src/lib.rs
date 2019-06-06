#[macro_use]
mod macros;

///
/// Handles server configuration.
///
pub mod config;

///
/// Handles serverside game logic.
///
pub mod game;

///
/// Handles serverside logic unrelated to the game or message transport method.
///
pub mod server;

///
/// Serves the client and handles websocket connections.
///
pub mod web;

use crate::config::Config;
use crate::server::Server as WebtronServer;
use actix::{Actor, System};
use failure::{Error, ResultExt};

///
/// Sets up environment, creates event loop,
/// starts Actix actors and runs event loop.
///
pub fn start() -> Result<(), Error> {
    pretty_env_logger::init();

    let config = Config::new();

    let system = System::new("webtron");
    let server_addr = WebtronServer::new().start();
    web::start(server_addr, &config).context("Failed to start HttpServer")?;

    system
        .run()
        .context("Failed to start webtron actix system arbiter")?;

    Ok(())
}
