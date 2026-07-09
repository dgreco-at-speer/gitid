//! Credential provisioning for `gitid new`: generate SSH keypairs and GPG keys,
//! and drive the isolated GitHub CLI (`gh`) for auth and key upload.
//!
//! Unlike [`crate::ssh`] (which only *discovers* existing keys), everything here
//! shells out to create new material. There is no central command runner in this
//! crate, so each function builds a [`std::process::Command`] directly.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::cli::SshKeyType;
use crate::paths::expand_tilde;

/// Whether `cmd` is on PATH. Probed by spawning it with `--version`: the exit
/// status is ignored (some tools, e.g. `ssh-keygen`, reject the flag) — a
/// successful spawn is what proves the binary exists. Only a "not found" spawn
/// error yields `false`.
pub fn have(cmd: &str) -> bool {
    Command::new(cmd).arg("--version").output().is_ok()
}

/// Outcome of an SSH-key provisioning step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshKeyOutcome {
    /// A new keypair was generated at the requested path.
    Generated,
    /// A key already existed at the path and was reused untouched.
    Reused,
}

/// Ensure an SSH keypair exists at `key` (a raw path that may contain `~`).
///
/// If a private key already exists there it is reused (never clobbered).
/// Otherwise `ssh-keygen -t <algo> -C <comment> -f <key>` is run. When
/// `interactive`, stdio is inherited so ssh-keygen prompts for a passphrase;
/// otherwise an empty passphrase (`-N ""`) is used so the call never blocks.
pub fn generate_ssh_key(
    key: &str,
    home: &Path,
    comment: &str,
    algo: SshKeyType,
    interactive: bool,
) -> Result<SshKeyOutcome> {
    let path = expand_tilde(key, home);
    if path.exists() {
        return Ok(SshKeyOutcome::Reused);
    }
    if !have("ssh-keygen") {
        bail!("ssh-keygen not found on PATH; install OpenSSH or pass an existing key");
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
        set_ssh_dir_perms(parent);
    }

    let mut cmd = Command::new("ssh-keygen");
    cmd.arg("-t")
        .arg(algo.as_str())
        .arg("-C")
        .arg(comment)
        .arg("-f")
        .arg(&path);
    if !interactive {
        // No passphrase, and never prompt (e.g. if the file somehow exists).
        cmd.arg("-N").arg("").arg("-q");
    }

    let status = cmd.status().context("could not run ssh-keygen")?;
    if !status.success() {
        bail!("ssh-keygen failed to generate {}", path.display());
    }
    Ok(SshKeyOutcome::Generated)
}

/// Restrict `~/.ssh` to owner-only, matching what ssh-keygen expects. No-op on
/// non-unix platforms.
#[cfg(unix)]
fn set_ssh_dir_perms(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(dir) {
        let mut perms = meta.permissions();
        perms.set_mode(0o700);
        let _ = std::fs::set_permissions(dir, perms);
    }
}

#[cfg(not(unix))]
fn set_ssh_dir_perms(_dir: &Path) {}

/// Generate an OpenPGP signing key non-interactively and return its fingerprint
/// (suitable for git's `user.signingkey`).
///
/// Uses `gpg --quick-generate-key "<name> <email>" ed25519 sign 1y`, then reads
/// the fingerprint back from `--list-secret-keys --with-colons`.
pub fn generate_gpg_key(name: &str, email: &str) -> Result<String> {
    if !have("gpg") {
        bail!("gpg not found on PATH; install GnuPG or supply an existing key id");
    }
    let uid = format!("{name} <{email}>");
    let status = Command::new("gpg")
        .args(["--batch", "--quick-generate-key"])
        .arg(&uid)
        .args(["ed25519", "sign", "1y"])
        .status()
        .context("could not run gpg --quick-generate-key")?;
    if !status.success() {
        bail!("gpg failed to generate a key for {uid:?}");
    }

    let out = Command::new("gpg")
        .args(["--list-secret-keys", "--with-colons"])
        .arg(email)
        .output()
        .context("could not run gpg --list-secret-keys")?;
    if !out.status.success() {
        bail!("gpg could not list the newly generated key for {email:?}");
    }
    let text = String::from_utf8_lossy(&out.stdout);
    parse_gpg_fingerprint(&text)
        .ok_or_else(|| anyhow::anyhow!("could not read fingerprint of the generated GPG key"))
}

/// Extract the fingerprint of the newest secret key from `gpg --with-colons`
/// output: the first `fpr:` line following a `sec:` record.
fn parse_gpg_fingerprint(colons: &str) -> Option<String> {
    let mut in_sec = false;
    for line in colons.lines() {
        let mut fields = line.split(':');
        match fields.next() {
            Some("sec") => in_sec = true,
            Some("fpr") if in_sec => {
                // Field 10 (index 9) holds the fingerprint.
                if let Some(fpr) = fields.nth(8) {
                    if !fpr.is_empty() {
                        return Some(fpr.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Run `gh auth login` against the profile's isolated `GH_CONFIG_DIR`,
/// inheriting stdio so the user completes the interactive flow.
pub fn gh_auth_login(gh_dir: &Path) -> Result<()> {
    run_gh(gh_dir, &["auth", "login"])
}

/// Upload an SSH public key to GitHub via the isolated `gh`.
pub fn gh_upload_ssh_key(gh_dir: &Path, pub_key: &Path, title: &str) -> Result<()> {
    run_gh(
        gh_dir,
        &[
            "ssh-key",
            "add",
            &pub_key.to_string_lossy(),
            "--title",
            title,
        ],
    )
}

/// Export a GPG public key and upload it to GitHub via the isolated `gh`.
pub fn gh_upload_gpg_key(gh_dir: &Path, key_id: &str) -> Result<()> {
    let exported = Command::new("gpg")
        .args(["--armor", "--export"])
        .arg(key_id)
        .output()
        .context("could not export GPG public key")?;
    if !exported.status.success() {
        bail!("gpg --export failed for key {key_id}");
    }
    let mut child = gh_command(gh_dir, &["gpg-key", "add"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("could not run gh gpg-key add")?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(&exported.stdout)
            .context("could not pipe GPG key to gh")?;
    }
    let status = child.wait().context("gh gpg-key add did not complete")?;
    if !status.success() {
        bail!("gh gpg-key add failed");
    }
    Ok(())
}

fn gh_command(gh_dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("gh");
    cmd.env("GH_CONFIG_DIR", gh_dir).args(args);
    cmd
}

fn run_gh(gh_dir: &Path, args: &[&str]) -> Result<()> {
    let status = gh_command(gh_dir, args)
        .status()
        .with_context(|| format!("could not run gh {}", args.join(" ")))?;
    if !status.success() {
        bail!("gh {} failed", args.join(" "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fingerprint_from_first_sec() {
        let colons = "\
sec:u:255:22:ABCDEF0123456789:1700000000::::::sc::::::23::0:
fpr:::::::::ABCDEF0123456789ABCDEF0123456789ABCDEF01:
uid:u::::1700000000::HASH::Jane <jane@example.com>::::::::::0:
ssb:u:255:22:1122334455667788:1700000000::::::s::::::23:
fpr:::::::::1122334455667788112233445566778811223344:
";
        assert_eq!(
            parse_gpg_fingerprint(colons).as_deref(),
            Some("ABCDEF0123456789ABCDEF0123456789ABCDEF01")
        );
    }

    #[test]
    fn no_fingerprint_when_absent() {
        assert!(parse_gpg_fingerprint("tru::1:1700000000:0:3:1:5\n").is_none());
    }
}
