//! Interactive prompts for `gitid add`. Falls back to flag values as defaults.

use std::fmt;

use anyhow::{Result, bail};
use inquire::{Confirm, Select, Text};

use crate::cli::AddArgs;
use crate::cmd::Ctx;
use crate::ssh::SshKey;
use crate::store::profiles::{Gh, Profile, Signing, SigningFormat, Ssh};

const NAV_HELP: &str = "type to filter · ↑↓ to move · enter to select";

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

/// One entry in the SSH-key picker: a discovered key or one of two sentinels.
enum SshChoice {
    Key(SshKey),
    None,
    Enter,
}

impl fmt::Display for SshChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SshChoice::Key(k) => write!(f, "{k}"),
            SshChoice::None => write!(f, "(none)"),
            SshChoice::Enter => write!(f, "(enter a path…)"),
        }
    }
}

fn prompt_ssh_key(ctx: &Ctx, args: &AddArgs) -> Result<Option<Ssh>> {
    if let Some(key) = &args.ssh_key {
        return Ok(Some(Ssh { key: key.clone() }));
    }
    let mut options: Vec<SshChoice> = crate::ssh::discover(&ctx.paths.home)
        .into_iter()
        .map(SshChoice::Key)
        .collect();
    options.push(SshChoice::None);
    options.push(SshChoice::Enter);
    let page = options.len().clamp(3, 12);
    let choice = Select::new("SSH key for this identity:", options)
        .with_help_message(NAV_HELP)
        .with_page_size(page)
        .prompt()?;
    match choice {
        SshChoice::None => Ok(None),
        SshChoice::Enter => {
            let key = Text::new("Path to SSH private key:").prompt()?;
            Ok(Some(Ssh { key }))
        }
        SshChoice::Key(k) => Ok(Some(Ssh { key: k.path })),
    }
}

fn prompt_signing(ssh: Option<&Ssh>) -> Result<Option<Signing>> {
    let choice = Select::new("Commit signing:", vec!["none", "ssh", "openpgp"])
        .with_help_message(NAV_HELP)
        .prompt()?;
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
