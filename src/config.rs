use std::net::{IpAddr, SocketAddr};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(rename_all = "kebab-case")]
struct CliConfig {
    /// Sets the interface to bind to
    #[arg(short = 'b', long, default_value = "127.0.0.1", env = "BIND_ADDRESS")]
    bind_address: IpAddr,

    /// Sets the port to bind to
    #[arg(short = 'p', long, default_value = "3000", env = "PORT")]
    port: u16,
}

#[derive(Debug)]
pub struct Config {
    pub bind_address: SocketAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 3000),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        let cli_config = CliConfig::parse();

        Self {
            bind_address: SocketAddr::new(cli_config.bind_address, cli_config.port),
        }
    }
}
