use failure::Error;
use log::error;
use std::{process, thread};
use webtron::{config::Config, server::WebtronServer, web};

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let config = Config::new();

    let (webtron_server, server_tx) = WebtronServer::new();

    thread::spawn(move || {
        web::run(server_tx, &config).unwrap_or_else(|error| {
            error!("Failed to run webserver: {}", error);
            process::exit(1);
        })
    });

    webtron_server.run();

    Ok(())
}
