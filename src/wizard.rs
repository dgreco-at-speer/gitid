//! Interactive prompts for `gitid add`. Falls back to flag values as defaults.

use std::fmt;

use anyhow::{Result, bail};
use inquire::{Confirm, Select, Text};

use crate::cli::{AddArgs, NewArgs};
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

/// Build a profile interactively for `gitid new`, generating SSH/GPG keys as
/// chosen. Uses any supplied flags as defaults.
pub fn new_wizard(ctx: &Ctx, name: &str, args: &NewArgs) -> Result<Profile> {
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

    let ssh = prompt_new_ssh_key(ctx, name, &email, args)?;

    let signing = prompt_new_signing(&git_name, &email, ssh.as_ref())?;

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

/// One entry in the `gitid new` SSH picker: generate, an existing key, or a
/// sentinel.
enum NewSshChoice {
    Generate,
    Key(SshKey),
    Enter,
    None,
}

impl fmt::Display for NewSshChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NewSshChoice::Generate => write!(f, "generate a new key"),
            NewSshChoice::Key(k) => write!(f, "{k}"),
            NewSshChoice::Enter => write!(f, "(enter an existing path…)"),
            NewSshChoice::None => write!(f, "(none)"),
        }
    }
}

fn prompt_new_ssh_key(ctx: &Ctx, name: &str, email: &str, args: &NewArgs) -> Result<Option<Ssh>> {
    if args.no_ssh {
        return Ok(None);
    }
    // An explicit --ssh-key means "provision here", skipping the picker.
    if let Some(key) = &args.ssh_key {
        return Ok(Some(generate_at(ctx, name, email, key, args)?));
    }

    let mut options = vec![NewSshChoice::Generate];
    options.extend(
        crate::ssh::discover(&ctx.paths.home)
            .into_iter()
            .map(NewSshChoice::Key),
    );
    options.push(NewSshChoice::Enter);
    options.push(NewSshChoice::None);
    let page = options.len().clamp(3, 12);
    let choice = Select::new("SSH key for this identity:", options)
        .with_help_message(NAV_HELP)
        .with_page_size(page)
        .prompt()?;
    match choice {
        NewSshChoice::Generate => {
            let default = crate::cmd::new::default_ssh_key_path(name);
            let key = Text::new("Where to store the new SSH key:")
                .with_default(&default)
                .prompt()?;
            Ok(Some(generate_at(ctx, name, email, &key, args)?))
        }
        NewSshChoice::Enter => {
            let key = Text::new("Path to existing SSH private key:").prompt()?;
            Ok(Some(Ssh { key }))
        }
        NewSshChoice::None => Ok(None),
        NewSshChoice::Key(k) => Ok(Some(Ssh { key: k.path })),
    }
}

/// Generate (or reuse) a key at `key`, reporting the outcome, and return the Ssh.
fn generate_at(ctx: &Ctx, name: &str, email: &str, key: &str, args: &NewArgs) -> Result<Ssh> {
    use crate::provision::SshKeyOutcome;
    let comment = crate::cmd::new::ssh_comment(email, name);
    let outcome =
        crate::provision::generate_ssh_key(key, &ctx.paths.home, &comment, args.ssh_type, true)?;
    match outcome {
        SshKeyOutcome::Generated => crate::output::success(&format!("generated SSH key {key}")),
        SshKeyOutcome::Reused => {
            crate::output::info(&format!("reusing existing SSH key {key} (not overwritten)"))
        }
    }
    Ok(Ssh {
        key: key.to_string(),
    })
}

fn prompt_new_signing(git_name: &str, email: &str, ssh: Option<&Ssh>) -> Result<Option<Signing>> {
    let choice = Select::new("Commit signing:", vec!["ssh", "openpgp", "none"])
        .with_help_message(NAV_HELP)
        .prompt()?;
    let format = match choice {
        "none" => return Ok(None),
        "ssh" => SigningFormat::Ssh,
        "openpgp" => SigningFormat::Openpgp,
        _ => unreachable!(),
    };
    let key = match format {
        SigningFormat::Ssh => {
            let default = ssh.map(|s| format!("{}.pub", s.key)).unwrap_or_default();
            Text::new("Path to SSH public signing key:")
                .with_default(&default)
                .prompt()?
        }
        SigningFormat::Openpgp => {
            let generate = Confirm::new("Generate a new GPG key?")
                .with_default(true)
                .prompt()?;
            if generate {
                let id = crate::provision::generate_gpg_key(git_name, email)?;
                crate::output::success(&format!("generated GPG key {id}"));
                id
            } else {
                Text::new("OpenPGP signing key id:").prompt()?
            }
        }
    };
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
