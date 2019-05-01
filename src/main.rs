use failure::Error;
use webtron::{config::Config, server::WebtronServer, web2};

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let config = Config::new();

    let (webtron_server, server_tx) = WebtronServer::new();
    webtron_server.run_in_new_thread();

    web2::run(server_tx, &config);

    Ok(())
}
