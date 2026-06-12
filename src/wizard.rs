//! Interactive prompts for `gitid add`. Falls back to flag values as defaults.

use std::path::Path;

use anyhow::{Result, bail};
use inquire::{Confirm, Select, Text};

use crate::cli::AddArgs;
use crate::cmd::Ctx;
use crate::store::profiles::{Gh, Profile, Signing, SigningFormat, Ssh};

/// Build a profile interactively, using any supplied flags as defaults.
pub fn add_wizard(ctx: &Ctx, args: &AddArgs) -> Result<Profile> {
    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        bail!("not a terminal; pass --non-interactive with --git-name and --email");
    }

    let git_name = match &args.git_name {
        Some(v) => v.clone(),
        None => Text::new("Git user.name:").prompt()?,
    };
    let email = match &args.email {
        Some(v) => v.clone(),
        None => Text::new("Git user.email:").prompt()?,
    };

    let ssh = prompt_ssh_key(ctx, args)?;

    let signing = prompt_signing(ssh.as_ref())?;

    let gh = if args.no_gh {
        None
    } else {
        let enabled = Confirm::new("Isolate GitHub CLI auth for this profile?")
            .with_default(true)
            .prompt()?;
        enabled.then_some(Gh { enabled: true })
    };

    Ok(Profile {
        name: git_name,
        email,
        ssh,
        signing,
        gh,
        env: Default::default(),
        extra: Default::default(),
    })
}

fn prompt_ssh_key(ctx: &Ctx, args: &AddArgs) -> Result<Option<Ssh>> {
    if let Some(key) = &args.ssh_key {
        return Ok(Some(Ssh { key: key.clone() }));
    }
    let mut options = discover_ssh_keys(&ctx.paths.home);
    options.push("(none)".to_string());
    options.push("(enter a path)".to_string());
    let choice = Select::new("SSH key for this identity:", options).prompt()?;
    match choice.as_str() {
        "(none)" => Ok(None),
        "(enter a path)" => {
            let key = Text::new("Path to SSH private key:").prompt()?;
            Ok(Some(Ssh { key }))
        }
        key => Ok(Some(Ssh {
            key: key.to_string(),
        })),
    }
}

fn prompt_signing(ssh: Option<&Ssh>) -> Result<Option<Signing>> {
    let choice = Select::new("Commit signing:", vec!["none", "ssh", "openpgp"]).prompt()?;
    let format = match choice {
        "none" => return Ok(None),
        "ssh" => SigningFormat::Ssh,
        "openpgp" => SigningFormat::Openpgp,
        _ => unreachable!(),
    };
    let default_key = match format {
        SigningFormat::Ssh => ssh.map(|s| format!("{}.pub", s.key)).unwrap_or_default(),
        SigningFormat::Openpgp => String::new(),
    };
    let prompt = match format {
        SigningFormat::Ssh => "Path to SSH public signing key:",
        SigningFormat::Openpgp => "OpenPGP signing key id:",
    };
    let key = Text::new(prompt).with_default(&default_key).prompt()?;
    let commits = Confirm::new("Sign commits by default?")
        .with_default(true)
        .prompt()?;
    Ok(Some(Signing {
        format,
        key,
        commits,
        tags: None,
    }))
}

/// List candidate private keys in `~/.ssh` (files starting with `id_`, excluding
/// `.pub`).
fn discover_ssh_keys(home: &Path) -> Vec<String> {
    let mut keys = Vec::new();
    let ssh_dir = home.join(".ssh");
    if let Ok(entries) = std::fs::read_dir(&ssh_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("id_") && !name.ends_with(".pub") {
                keys.push(entry.path().to_string_lossy().into_owned());
            }
        }
    }
    keys.sort();
    keys
}
