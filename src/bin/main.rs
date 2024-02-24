use busy_bee::cli::{Cli, Commands};
use clap::Parser;

fn main() {
    let args = Cli::parse();
    match args.command {
        _ => todo!(),
    }
}
