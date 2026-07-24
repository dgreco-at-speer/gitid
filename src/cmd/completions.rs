//! `gitid completions <shell>` and the hidden `gitid __complete` backend.
//!
//! Completion scripts are dynamic: the static part (subcommand names, flags)
//! lives in the per-shell template and is routed by the shell itself; profile
//! names are resolved at completion time by shelling back out to
//! `gitid __complete profiles <prefix>`. Directory completion uses each
//! shell's native machinery — no backend involvement.

use std::process::ExitCode;

use anyhow::Result;

use crate::cli::{CompleteArgs, CompletionsArgs, Shell};
use crate::cmd::Ctx;
use crate::store::profiles;

const BASH: &str = include_str!("../shell/templates/complete.bash");
const ZSH: &str = include_str!("../shell/templates/complete.zsh");
const FISH: &str = include_str!("../shell/templates/complete.fish");
const PS1: &str = include_str!("../shell/templates/complete.ps1");
const NU: &str = include_str!("../shell/templates/complete.nu");

/// The completion script for `shell`, ready to eval/source.
pub(crate) fn script(shell: Shell) -> String {
    let template = match shell {
        Shell::Bash => BASH,
        Shell::Zsh => ZSH,
        Shell::Fish => FISH,
        Shell::Powershell => PS1,
        Shell::Nu => NU,
    };
    template.replace("{{GITID}}", "gitid")
}

pub fn run(args: &CompletionsArgs) -> Result<()> {
    print!("{}", script(args.shell));
    Ok(())
}

/// `gitid __complete` — the machine-facing backend the shell templates call
/// into for dynamic candidates. Best-effort, like `gitid env`: on any failure
/// (no store, corrupt file) print nothing and exit 0 so a completion request
/// never breaks the prompt.
pub fn complete(ctx: &Ctx, args: &CompleteArgs) -> Result<ExitCode> {
    if args.what == "profiles" {
        let prefix = args.current.as_deref().unwrap_or("");
        if let Ok(p) = profiles::load(&ctx.paths.profiles_toml()) {
            for name in p.profiles.keys() {
                if name.starts_with(prefix) {
                    println!("{name}");
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
