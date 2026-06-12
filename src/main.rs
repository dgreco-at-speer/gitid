use std::process::ExitCode;

use clap::Parser;

use gitid::cli::Cli;
use gitid::output;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match gitid::cmd::run(cli) {
        Ok(code) => code,
        Err(err) => {
            output::error(&format!("{err:#}"));
            ExitCode::FAILURE
        }
    }
}
