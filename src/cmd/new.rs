//! `gitid new` — provision a profile from scratch, generating SSH/GPG keys and
//! (optionally) driving the isolated GitHub CLI. Contrast with `gitid add`,
//! which only references credentials that already exist.

use anyhow::{Result, bail};

use crate::cli::{NewArgs, SigningKind};
use crate::cmd::Ctx;
use crate::output;
use crate::paths::expand_tilde;
use crate::provision::{self, SshKeyOutcome};
use crate::store::profiles::{self, Gh, Profile, Signing, SigningFormat, Ssh};
use crate::sync::sync_all;

pub fn run(ctx: &Ctx, args: &NewArgs) -> Result<()> {
    let name = resolve_name(args)?;
    profiles::validate_name(&name)?;

    let path = ctx.paths.profiles_toml();
    let existing = profiles::load(&path)?;
    if existing.profiles.contains_key(&name) {
        bail!("profile {name:?} already exists; use `gitid edit {name}`");
    }

    let profile = if args.non_interactive {
        build_non_interactive(ctx, &name, args)?
    } else {
        crate::wizard::new_wizard(ctx, &name, args)?
    };

    let mut doc = profiles::load_doc(&path)?;
    profiles::upsert_profile(&mut doc, &name, &profile)?;
    profiles::save_doc(&path, &doc)?;

    let report = output::with_spinner("syncing…", || sync_all(&ctx.paths))?;
    output::success(&format!("created profile {name:?}"));
    crate::cmd::sync::report_changes(&report);

    // `sync_all` created the gh config dir; offer GitHub setup last (interactive
    // only), and always leave copy-pasteable next steps behind.
    if !args.non_interactive {
        github_followup(ctx, &name, &profile)?;
    }
    next_steps(ctx, &name, &profile);
    Ok(())
}

fn resolve_name(args: &NewArgs) -> Result<String> {
    if let Some(n) = &args.name {
        return Ok(n.clone());
    }
    if !args.non_interactive && std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        return Ok(inquire::Text::new("Profile name (slug):").prompt()?);
    }
    bail!("a profile name is required (e.g. `gitid new work ...`)")
}

/// Build a profile purely from flags, generating keys as directed. Takes no
/// interactive actions.
fn build_non_interactive(ctx: &Ctx, name: &str, args: &NewArgs) -> Result<Profile> {
    let git_name = args
        .git_name
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--git-name is required in non-interactive mode"))?;
    let email = args
        .email
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--email is required in non-interactive mode"))?;

    let ssh = if args.no_ssh {
        None
    } else if let Some(key) = &args.ssh_key {
        let comment = ssh_comment(&email, name);
        let outcome =
            provision::generate_ssh_key(key, &ctx.paths.home, &comment, args.ssh_type, false)?;
        report_ssh(key, outcome);
        Some(Ssh::from_path(key.clone()))
    } else {
        None
    };

    let signing = match args.signing {
        None | Some(SigningKind::None) => None,
        Some(kind) => Some(build_signing(&git_name, &email, kind, args, ssh.as_ref())?),
    };

    let gh = if args.no_gh {
        None
    } else {
        Some(Gh { enabled: true })
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

/// Assemble a [`Signing`] for a chosen method, generating an OpenPGP key when
/// one is required and not supplied. `SigningKind::None` must be filtered first.
fn build_signing(
    git_name: &str,
    email: &str,
    kind: SigningKind,
    args: &NewArgs,
    ssh: Option<&Ssh>,
) -> Result<Signing> {
    let (format, key) = match kind {
        SigningKind::None => unreachable!("None is filtered by the caller"),
        SigningKind::Ssh => {
            let key = match &args.signing_key {
                Some(k) => k.clone(),
                None => {
                    let key = ssh.and_then(Ssh::path).ok_or_else(|| {
                        anyhow::anyhow!(
                            "--signing ssh needs an SSH key: pass --signing-key or drop --no-ssh"
                        )
                    })?;
                    format!("{key}.pub")
                }
            };
            (SigningFormat::Ssh, key)
        }
        SigningKind::Openpgp => {
            let key = match &args.signing_key {
                Some(k) => k.clone(),
                None => provision::generate_gpg_key(git_name, email)?,
            };
            (SigningFormat::Openpgp, key)
        }
    };
    Ok(Signing {
        format,
        key,
        commits: args.sign_commits,
        tags: None,
    })
}

/// Default `ssh-keygen` comment: the email, falling back to the profile slug.
pub fn ssh_comment(email: &str, name: &str) -> String {
    if email.trim().is_empty() {
        name.to_string()
    } else {
        email.to_string()
    }
}

/// The conventional default private-key path for a profile.
pub fn default_ssh_key_path(name: &str) -> String {
    format!("~/.ssh/id_ed25519_{name}")
}

/// Print whether a key was generated or an existing one reused.
fn report_ssh(key: &str, outcome: SshKeyOutcome) {
    match outcome {
        SshKeyOutcome::Generated => output::success(&format!("generated SSH key {key}")),
        SshKeyOutcome::Reused => {
            output::info(&format!("reusing existing SSH key {key} (not overwritten)"));
        }
    }
}

/// Interactive GitHub follow-up: offer to authenticate and upload keys against
/// the profile's isolated `GH_CONFIG_DIR`.
fn github_followup(ctx: &Ctx, name: &str, profile: &Profile) -> Result<()> {
    if !profile.gh_enabled() {
        return Ok(());
    }
    let gh_dir = ctx.paths.gh_dir(name);
    if !provision::have("gh") {
        output::info("install the GitHub CLI (`gh`) to authenticate this profile");
        return Ok(());
    }

    if inquire::Confirm::new("Authenticate GitHub CLI for this profile now?")
        .with_default(false)
        .prompt()?
    {
        if let Err(e) = provision::gh_auth_login(&gh_dir) {
            output::warn(&format!("gh auth login: {e:#}"));
        }
    }

    if let Some(key) = profile.ssh.as_ref().and_then(Ssh::path) {
        let pub_key = expand_tilde(&format!("{key}.pub"), &ctx.paths.home);
        if pub_key.exists()
            && inquire::Confirm::new("Upload the SSH public key to GitHub?")
                .with_default(false)
                .prompt()?
        {
            let title = format!("gitid:{name}");
            if let Err(e) = provision::gh_upload_ssh_key(&gh_dir, &pub_key, &title) {
                output::warn(&format!("gh ssh-key add: {e:#}"));
            }
        }
    }

    if let Some(signing) = &profile.signing {
        if signing.format == SigningFormat::Openpgp
            && inquire::Confirm::new("Upload the GPG public key to GitHub?")
                .with_default(false)
                .prompt()?
        {
            if let Err(e) = provision::gh_upload_gpg_key(&gh_dir, &signing.key) {
                output::warn(&format!("gh gpg-key add: {e:#}"));
            }
        }
    }
    Ok(())
}

/// Print copy-pasteable next steps so a declined follow-up still leaves guidance.
fn next_steps(ctx: &Ctx, name: &str, profile: &Profile) {
    let ssh_path = profile.ssh.as_ref().and_then(Ssh::path);
    if let Some(key) = ssh_path {
        output::info(&format!("SSH public key: {key}.pub"));
    }
    if profile.gh_enabled() {
        let gh_dir = ctx.paths.gh_dir(name);
        output::hint(&format!(
            "authenticate GitHub: GH_CONFIG_DIR={} gh auth login",
            gh_dir.display()
        ));
        if let Some(key) = ssh_path {
            output::hint(&format!(
                "upload SSH key:      GH_CONFIG_DIR={} gh ssh-key add {key}.pub",
                gh_dir.display()
            ));
        }
    }
    output::info(&format!("assign a directory: gitid use {name} <dir>"));
}
