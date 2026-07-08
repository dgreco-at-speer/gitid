use std::process::ExitCode;

use clap::Parser;

use gitid::cli::Cli;
use gitid::output;

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.no_color {
        output::set_color_override(false);
    } else {
        output::init_color();
    }
    output::init_prompts();
    match gitid::cmd::run(cli) {
        Ok(code) => code,
        Err(err) => {
            output::error(&format!("{err:#}"));
            ExitCode::FAILURE
        }
    }
}
