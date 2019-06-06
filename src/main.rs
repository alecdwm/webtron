use failure::Error;
use log::error;
use std::{env, process};

fn main() {
    webtron::start().unwrap_or_else(|error| {
        print_error_chain(error);
        process::exit(1);
    })
}

fn print_error_chain(error: Error) {
    let chain = error
        .iter_chain()
        .map(|f| format!("{}", f))
        .collect::<Vec<_>>()
        .join(": ");

    if env::var("RUST_BACKTRACE").is_ok() {
        error!("{}\n\n{}", error.backtrace(), chain);
    } else {
        error!("{}", chain);
    }
}
