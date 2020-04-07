use log::error;
use std::env;
use webtron::get_error_chain;

#[tokio::main]
#[quit::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "webtron=trace");
    }
    pretty_env_logger::init();

    webtron::start().await.unwrap_or_else(|error| {
        error!("{}", get_error_chain(error));
        quit::with_code(1);
    });
}
