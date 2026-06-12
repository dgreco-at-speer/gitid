//! `gitid completions <shell>`.

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::{Cli, CompletionsArgs, Shell};

pub fn run(args: &CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    let mut out = std::io::stdout();
    match args.shell {
        Shell::Bash => generate(clap_complete::Shell::Bash, &mut cmd, "gitid", &mut out),
        Shell::Zsh => generate(clap_complete::Shell::Zsh, &mut cmd, "gitid", &mut out),
        Shell::Fish => generate(clap_complete::Shell::Fish, &mut cmd, "gitid", &mut out),
        Shell::Powershell => generate(
            clap_complete::Shell::PowerShell,
            &mut cmd,
            "gitid",
            &mut out,
        ),
        Shell::Nu => generate(clap_complete_nushell::Nushell, &mut cmd, "gitid", &mut out),
    }
    Ok(())
}
