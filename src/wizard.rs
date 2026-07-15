//! Interactive prompts for `gitid add`. Falls back to flag values as defaults.

use std::fmt;

use anyhow::{Result, bail};
use inquire::{Confirm, Select, Text};

use crate::agent::AgentKey;
use crate::cli::AddArgs;
use crate::cmd::Ctx;
use crate::ssh::SshKey;
use crate::store::profiles::{Gh, Profile, SIGNING_KEY_AGENT, Signing, SigningFormat, Ssh};

const NAV_HELP: &str = "type to filter · ↑↓ to move · enter to select";

/// Build a profile interactively, using any supplied flags as defaults.
pub fn add_wizard(ctx: &Ctx, name: &str, args: &AddArgs) -> Result<Profile> {
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

    let ssh = prompt_ssh_key(ctx, name, &email, args)?;

    let signing = prompt_signing(&git_name, &email, ssh.as_ref())?;

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

/// One entry in the SSH-key picker: generate a new key, a discovered key file,
/// a key held by the ssh-agent, a manually-entered path, or none.
enum SshChoice {
    Generate,
    Key(SshKey),
    Agent(AgentKey),
    Enter,
    None,
}

impl fmt::Display for SshChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SshChoice::Generate => write!(f, "generate a new key"),
            SshChoice::Key(k) => write!(f, "{}", k),
            SshChoice::Agent(k) => {
                let comment = k.public.comment();
                let label = if comment.is_empty() {
                    k.fingerprint()
                } else {
                    comment.to_string()
                };
                write!(f, "agent: {label}")
            }
            SshChoice::Enter => write!(f, "enter a path…"),
            SshChoice::None => write!(f, "none"),
        }
    }
}

fn prompt_ssh_key(ctx: &Ctx, name: &str, email: &str, args: &AddArgs) -> Result<Option<Ssh>> {
    if args.no_ssh {
        return Ok(None);
    }
    // An explicit --ssh-key means "provision here", skipping the picker.
    if let Some(key) = &args.ssh_key {
        return Ok(Some(generate_at(ctx, name, email, key, args)?));
    }
    if let Some(selector) = &args.ssh_agent_key {
        crate::agent::validate_selector(selector)?;
        return Ok(Some(Ssh::from_agent(selector.clone())));
    }

    let mut options = vec![SshChoice::Generate];
    options.extend(
        crate::ssh::discover(&ctx.paths.home)
            .into_iter()
            .map(SshChoice::Key),
    );
    // Best-effort: no running agent just means no agent entries.
    options.extend(
        crate::agent::list_keys()
            .unwrap_or_default()
            .into_iter()
            .map(SshChoice::Agent),
    );
    options.push(SshChoice::Enter);
    options.push(SshChoice::None);
    let page = options.len().clamp(3, 12);
    let choice = Select::new("SSH key for this identity:", options)
        .with_help_message(NAV_HELP)
        .with_page_size(page)
        .prompt()?;
    match choice {
        SshChoice::Generate => {
            let default = crate::cmd::add::default_ssh_key_path(name);
            let key = Text::new("Where to store the new SSH key:")
                .with_default(&default)
                .prompt()?;
            Ok(Some(generate_at(ctx, name, email, &key, args)?))
        }
        SshChoice::Enter => {
            let key = Text::new("Path to existing SSH private key:").prompt()?;
            Ok(Some(Ssh::from_path(key)))
        }
        SshChoice::None => Ok(None),
        SshChoice::Key(k) => Ok(Some(Ssh::from_path(k.path))),
        // The fingerprint is the canonical selector: comments may be empty or
        // shared between keys.
        SshChoice::Agent(k) => Ok(Some(Ssh::from_agent(k.fingerprint()))),
    }
}

/// Generate (or reuse) a key at `key`, reporting the outcome, and return the Ssh.
fn generate_at(ctx: &Ctx, name: &str, email: &str, key: &str, args: &AddArgs) -> Result<Ssh> {
    use crate::provision::SshKeyOutcome;
    let comment = crate::cmd::add::ssh_comment(email, name);
    let outcome =
        crate::provision::generate_ssh_key(key, &ctx.paths.home, &comment, args.ssh_type, true)?;
    match outcome {
        SshKeyOutcome::Generated => crate::output::success(&format!("generated SSH key {key}")),
        SshKeyOutcome::Reused => {
            crate::output::info(&format!("reusing existing SSH key {key} (not overwritten)"))
        }
    }
    Ok(Ssh::from_path(key))
}

fn prompt_signing(git_name: &str, email: &str, ssh: Option<&Ssh>) -> Result<Option<Signing>> {
    let choice = Select::new("Commit signing:", vec!["none", "ssh", "openpgp"])
        .with_help_message(NAV_HELP)
        .prompt()?;
    let format = match choice {
        "none" => return Ok(None),
        "ssh" => SigningFormat::Ssh,
        "openpgp" => SigningFormat::Openpgp,
        _ => unreachable!(),
    };
    // An agent-held key signs via the "agent" sentinel; no prompt needed.
    if format == SigningFormat::Ssh && ssh.is_some_and(|s| s.agent_selector().is_some()) {
        let commits = Confirm::new("Sign commits by default?")
            .with_default(true)
            .prompt()?;
        return Ok(Some(Signing {
            format,
            key: SIGNING_KEY_AGENT.to_string(),
            commits,
            tags: None,
        }));
    }
    let key = match format {
        SigningFormat::Ssh => {
            let default = ssh
                .and_then(|s| s.path())
                .map(|p| format!("{p}.pub"))
                .unwrap_or_default();
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
