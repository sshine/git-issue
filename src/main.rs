use clap::Parser;
use env_logger::Env;

mod cli;
mod common;
mod storage;

use cli::{Cli, run_command};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    if let Err(err) = run_command(cli) {
        eprintln!("{}", cli::error_message(&err.to_string()));
        std::process::exit(1);
    }
}
