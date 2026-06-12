//! `gitid setup` — install the shell hook into the user's rc file (with consent).

use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::cli::{SetupArgs, Shell};
use crate::cmd::Ctx;
use crate::output;
use crate::shell::hook;

const BEGIN: &str = "# >>> gitid hook >>>";
const END: &str = "# <<< gitid hook <<<";

pub fn run(ctx: &Ctx, args: &SetupArgs) -> Result<()> {
    let shell = match args.shell {
        Some(s) => s,
        None => detect_shell()
            .context("could not detect your shell; pass one explicitly, e.g. `gitid setup zsh`")?,
    };

    let rc = ctx.paths.home.join(hook::rc_file(shell));

    // nushell can't eval; its hook lives in a standalone autoload file.
    if shell == Shell::Nu {
        return setup_nu(ctx, &rc, args);
    }

    let line = hook::install_line(shell);
    if args.print {
        println!("# add to {}", rc.display());
        println!("{line}");
        return Ok(());
    }

    let existing = std::fs::read_to_string(&rc).unwrap_or_default();
    if existing.contains(BEGIN) {
        output::success(&format!("hook already installed in {}", rc.display()));
        return Ok(());
    }

    output::info(&format!("will append the gitid hook to {}:", rc.display()));
    println!("    {line}");
    if !args.yes && !confirm(&format!("Append to {}?", rc.display()))? {
        output::info("skipped; add the line above yourself, then restart your shell");
        return Ok(());
    }

    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!("\n{BEGIN}\n{line}\n{END}\n"));
    write_with_parents(&rc, &content)?;
    output::success(&format!("installed gitid hook in {}", rc.display()));
    output::info("restart your shell (or source the rc file) to activate");
    Ok(())
}

fn setup_nu(ctx: &Ctx, rc: &std::path::Path, args: &SetupArgs) -> Result<()> {
    let script = hook::script(Shell::Nu);
    if args.print {
        println!("# save to {}", rc.display());
        print!("{script}");
        return Ok(());
    }
    output::info(&format!(
        "will write the gitid nushell hook to {}",
        rc.display()
    ));
    if !args.yes && !confirm(&format!("Write {}?", rc.display()))? {
        output::info("skipped");
        return Ok(());
    }
    let _ = ctx;
    write_with_parents(rc, &script)?;
    output::success(&format!("wrote gitid hook to {}", rc.display()));
    output::info("restart nushell to activate");
    Ok(())
}

fn write_with_parents(path: &std::path::Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("could not write {}", path.display()))
}

fn confirm(prompt: &str) -> Result<bool> {
    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        bail!("not a terminal; re-run with --yes to append, or --print to see the line");
    }
    inquire::Confirm::new(prompt)
        .with_default(false)
        .prompt()
        .map_err(Into::into)
}

/// Detect the user's shell from `$SHELL` (or assume PowerShell on Windows).
fn detect_shell() -> Option<Shell> {
    if cfg!(windows) {
        return Some(Shell::Powershell);
    }
    let shell = std::env::var("SHELL").ok()?;
    let base = PathBuf::from(shell);
    let name = base.file_name()?.to_string_lossy().to_lowercase();
    if name.contains("bash") {
        Some(Shell::Bash)
    } else if name.contains("zsh") {
        Some(Shell::Zsh)
    } else if name.contains("fish") {
        Some(Shell::Fish)
    } else if name.contains("nu") {
        Some(Shell::Nu)
    } else if name.contains("pwsh") || name.contains("powershell") {
        Some(Shell::Powershell)
    } else {
        None
    }
}
