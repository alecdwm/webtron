use std::net::{IpAddr, SocketAddr};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct CliConfig {
    /// Sets the interface to bind to
    #[structopt(short = "b", long, default_value = "127.0.0.1", env = "BIND_ADDRESS")]
    bind_address: IpAddr,

    /// Sets the port to bind to
    #[structopt(short = "p", long, default_value = "3000", env = "PORT")]
    port: u16,
}

#[derive(Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
}

impl Config {
    pub fn new() -> Self {
        let cli_config = CliConfig::from_args();

        Self {
            bind_addr: SocketAddr::new(cli_config.bind_address, cli_config.port),
        }
    }
}
