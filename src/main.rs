#[macro_use]
extern crate log;

use std::env;
use std::io::Write;

use env_logger::Env;

mod api;
mod cli;
mod entities;
mod errors;
mod storage;

fn main() {
    initialize_logging();

    // Parse CLI args and dispatch to the right subcommand
    let result = cli::run();

    // If there is an error, print it and exit with a non-zero error code
    cli::print_error(result);
}

fn initialize_logging() {
    // If the RUST_LOG environment variable is defined, respect it and use the default formatter.
    // Otherwise fallback onto "warn" level and a minimalistic formatter, to allow outputting
    // warnings on the console.
    if env::var("RUST_LOG").is_err() {
        env_logger::from_env(Env::default().default_filter_or("tmsu=warn"))
            .format(|buf, record| writeln!(buf, "tmsu: {}", record.args()))
            .init();
    } else {
        env_logger::init();
    }
}
