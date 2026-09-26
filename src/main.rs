use log::error;
use std::env;
use webtron::get_error_chain;

/// What is logged when `RUST_LOG` isn't set.
const DEFAULT_LOG_FILTERS: &str = "webtron=info";

#[tokio::main]
#[quit::main]
async fn main() {
    let filters = env::var("RUST_LOG").unwrap_or_else(|_| DEFAULT_LOG_FILTERS.to_string());
    pretty_env_logger::formatted_builder()
        .parse_filters(&filters)
        .init();

    webtron::start().await.unwrap_or_else(|error| {
        error!("{}", get_error_chain(error));
        quit::with_code(1);
    });
}
