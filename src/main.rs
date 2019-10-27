use log::error;
use std::process;
use webtron::get_error_chain;

fn main() {
    webtron::start().unwrap_or_else(|error| {
        error!("{}", get_error_chain(error));
        process::exit(1);
    })
}
