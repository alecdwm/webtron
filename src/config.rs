use clap::Parser;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(rename_all = "kebab-case")]
struct CliConfig {
    /// Sets the interface to bind to
    #[arg(short = 'b', long, default_value = "127.0.0.1", env = "BIND_ADDRESS")]
    bind_address: IpAddr,

    /// Sets the port to bind to
    #[arg(short = 'p', long, default_value = "3000", env = "PORT")]
    port: u16,

    /// Sets the path of the file that holds the leaderboard
    #[arg(
        short = 'l',
        long,
        default_value = "leaderboard.jsonl",
        env = "LEADERBOARD_PATH"
    )]
    leaderboard_path: PathBuf,

    /// Sets the ntfy topic url, such as https://ntfy.sh/mytopic, to post to
    /// when a player connects or disconnects
    #[arg(short = 'n', long, env = "NTFY_URL")]
    ntfy_url: Option<String>,
}

#[derive(Debug)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub leaderboard_path: PathBuf,
    pub ntfy_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 3000),
            leaderboard_path: PathBuf::from("leaderboard.jsonl"),
            ntfy_url: None,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        let cli_config = CliConfig::parse();

        Self {
            bind_address: SocketAddr::new(cli_config.bind_address, cli_config.port),
            leaderboard_path: cli_config.leaderboard_path,
            ntfy_url: cli_config.ntfy_url,
        }
    }
}
