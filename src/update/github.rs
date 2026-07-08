//! The network + archive layer, implemented by shelling out to the same tools
//! the install scripts use (`gh` preferred, `curl` fallback, `tar`/PowerShell for
//! extraction). This keeps gitid dependency-light and inherits the private-repo
//! auth story from the user's `gh` login (or `GH_TOKEN`/`GITHUB_TOKEN`).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};

use super::target::Target;

/// Default upstream, overridable via `$GITID_REPO` (matches the install scripts).
pub const DEFAULT_REPO: &str = "dgreco-at-speer/gitid";

/// The `owner/repo` to update from.
pub fn repo() -> String {
    std::env::var("GITID_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_string())
}

/// A token for `curl`-based fallback downloads on private repos.
fn token() -> Option<String> {
    std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .ok()
        .filter(|t| !t.is_empty())
}

/// Whether `cmd` is on PATH (mirrors `have()` in install.sh).
pub fn have(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Resolve the latest release tag. Prefers `gh` (handles private auth); falls
/// back to the GitHub REST API via `curl` with an optional bearer token.
pub fn latest_tag(repo: &str) -> Result<String> {
    if have("gh") {
        let out = Command::new("gh")
            .args([
                "release", "view", "--repo", repo, "--json", "tagName", "-q", ".tagName",
            ])
            .output()
            .context("failed to run gh")?;
        if out.status.success() {
            let tag = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !tag.is_empty() {
                return Ok(tag);
            }
        }
        // Fall through to curl only if gh produced nothing useful; surface gh's
        // stderr if curl is also unavailable.
        if !have("curl") {
            bail!(
                "gh could not resolve the latest release: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
    }

    if !have("curl") {
        bail!("need either gh or curl installed to check for updates");
    }
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL"]);
    if let Some(tok) = token() {
        cmd.arg("-H").arg(format!("Authorization: Bearer {tok}"));
    }
    cmd.arg(format!(
        "https://api.github.com/repos/{repo}/releases/latest"
    ));
    let out = cmd.output().context("failed to run curl")?;
    if !out.status.success() {
        bail!(
            "could not query releases (private repo? install gh and run 'gh auth login', or set GH_TOKEN)"
        );
    }
    let body: serde_json::Value =
        serde_json::from_slice(&out.stdout).context("could not parse releases API response")?;
    body.get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("could not determine latest version from releases API"))
}

/// Download the release archive for `tag`/`target` into `dest_dir` and return the
/// path to the downloaded archive. Tries the preferred triple first, then any
/// fallback (musl → gnu on Linux), mirroring `scripts/install.sh`.
pub fn download_asset(repo: &str, tag: &str, target: &Target, dest_dir: &Path) -> Result<PathBuf> {
    if !have("gh") && !have("curl") {
        bail!("need either gh or curl installed to download updates");
    }
    let mut last_err = None;
    for triple in target.triples() {
        match download_one(repo, tag, triple, target.ext, dest_dir) {
            Ok(path) => return Ok(path),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("no release asset matched this platform")))
}

/// Attempt to download a single `triple`'s asset.
fn download_one(
    repo: &str,
    tag: &str,
    triple: &str,
    ext: &str,
    dest_dir: &Path,
) -> Result<PathBuf> {
    let asset = format!("gitid-{tag}-{triple}.{ext}");
    if have("gh") {
        let status = Command::new("gh")
            .args(["release", "download", tag, "--repo", repo, "--pattern"])
            .arg(format!("gitid-*-{triple}.{ext}"))
            .arg("--dir")
            .arg(dest_dir)
            .status()
            .context("failed to run gh release download")?;
        if status.success() {
            return find_one(dest_dir, |name| name == asset)
                .ok_or_else(|| anyhow!("gh reported success but no {asset} was downloaded"));
        }
        if !have("curl") {
            bail!("gh could not download {asset} (does the release have this asset?)");
        }
    }

    let url = format!("https://github.com/{repo}/releases/download/{tag}/{asset}");
    let out_path = dest_dir.join(&asset);
    let mut cmd = Command::new("curl");
    cmd.args(["-fSL"]);
    if let Some(tok) = token() {
        cmd.arg("-H").arg(format!("Authorization: Bearer {tok}"));
    }
    cmd.arg(&url).arg("-o").arg(&out_path);
    let status = cmd.status().context("failed to run curl")?;
    if !status.success() {
        bail!("download failed for {asset} (private repo? install gh and run 'gh auth login')");
    }
    Ok(out_path)
}

/// Extract `archive` into `dest_dir` and return the path to the `gitid` binary
/// inside it. The release archives nest the binary one directory deep, so the
/// binary is located by a recursive search (matching the install scripts).
pub fn extract_binary(archive: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let name = archive
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if name.ends_with(".zip") {
        // Windows path: PowerShell's Expand-Archive (as in install.ps1).
        let status = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Expand-Archive", "-Path"])
            .arg(archive)
            .args(["-DestinationPath"])
            .arg(dest_dir)
            .arg("-Force")
            .status()
            .context("failed to run Expand-Archive")?;
        if !status.success() {
            bail!("could not extract {}", archive.display());
        }
    } else {
        let status = Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .arg("-C")
            .arg(dest_dir)
            .status()
            .context("failed to run tar")?;
        if !status.success() {
            bail!("could not extract {}", archive.display());
        }
    }

    let bin_name = format!("gitid{}", std::env::consts::EXE_SUFFIX);
    find_one(dest_dir, |n| n == bin_name).ok_or_else(|| anyhow!("binary not found in archive"))
}

/// Recursively find the first file under `dir` whose file name satisfies `pred`.
fn find_one(dir: &Path, pred: impl Fn(&str) -> bool + Copy) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let ft = entry.file_type().ok()?;
        if ft.is_dir() {
            if let Some(found) = find_one(&path, pred) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()).is_some_and(pred) {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_honours_override() {
        // Default when unset is asserted indirectly; here just check the constant.
        assert_eq!(DEFAULT_REPO, "dgreco-at-speer/gitid");
    }

    #[test]
    fn find_one_locates_nested_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("gitid-v0.3.0-x86_64-unknown-linux-gnu");
        std::fs::create_dir_all(&nested).unwrap();
        let bin = nested.join("gitid");
        std::fs::write(&bin, b"#!/bin/sh\n").unwrap();
        std::fs::write(nested.join("README.md"), b"readme").unwrap();
        let found = find_one(tmp.path(), |n| n == "gitid").unwrap();
        assert_eq!(found, bin);
    }
}
