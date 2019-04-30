use failure::Error;
use std::thread;
use webtron::{config::Config, game_server, web};

///
/// main.rs
///

fn main() -> Result<(), Error> {
    let config = Config::new();

    let (webtron_server, server_tx) = game_server::WebtronServer::new();
    webtron_server.run_in_thread();

    web::run(server_tx, &config);

    Ok(())
}
