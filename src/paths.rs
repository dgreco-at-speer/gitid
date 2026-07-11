//! Path helpers and platform directory resolution.
//!
//! The string-munging functions here are pure and take an explicit [`PathStyle`]
//! and `home` so that Windows-flavoured behaviour can be unit-tested from a Unix
//! host (and vice-versa). Only [`GitidPaths::resolve`] touches the real
//! environment.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use etcetera::BaseStrategy;

/// Separator convention to apply when normalising a path into a gitconfig
/// `gitdir:` pattern. Selected from the real platform at runtime, but overridable
/// in tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStyle {
    Unix,
    Windows,
}

impl PathStyle {
    /// The style matching the host this binary was compiled for.
    pub const fn host() -> Self {
        if cfg!(windows) {
            PathStyle::Windows
        } else {
            PathStyle::Unix
        }
    }

    /// Whether case-insensitive directory matching is the sensible default for
    /// this platform (Windows + macOS use case-insensitive filesystems by
    /// default; Linux does not).
    pub fn default_icase(self) -> bool {
        match self {
            PathStyle::Windows => true,
            PathStyle::Unix => cfg!(target_os = "macos"),
        }
    }
}

/// Resolved on-disk locations gitid reads and writes.
#[derive(Debug, Clone)]
pub struct GitidPaths {
    /// Config dir holding the hand-editable sources of truth (`profiles.toml`,
    /// `mappings.toml`).
    pub config_dir: PathBuf,
    /// Data dir holding generated artefacts and per-profile gh config dirs.
    pub data_dir: PathBuf,
    /// The user's home directory.
    pub home: PathBuf,
}

impl GitidPaths {
    /// Resolve paths from the environment.
    ///
    /// `$GITID_CONFIG_DIR` and `$GITID_DATA_DIR` override the platform defaults
    /// (used by the test-suite and power users). Otherwise the `etcetera` base
    /// strategy is used, which is XDG on Linux *and* macOS — matching where git
    /// itself looks — and Known Folders on Windows.
    pub fn resolve() -> Result<Self> {
        let strategy =
            etcetera::choose_base_strategy().context("could not determine home directory")?;
        let home = strategy.home_dir().to_path_buf();

        let config_dir = match std::env::var_os("GITID_CONFIG_DIR") {
            Some(p) => PathBuf::from(p),
            None => strategy.config_dir().join("gitid"),
        };
        let data_dir = match std::env::var_os("GITID_DATA_DIR") {
            Some(p) => PathBuf::from(p),
            None => strategy.data_dir().join("gitid"),
        };

        Ok(Self {
            config_dir,
            data_dir,
            home,
        })
    }

    pub fn profiles_toml(&self) -> PathBuf {
        self.config_dir.join("profiles.toml")
    }

    pub fn mappings_toml(&self) -> PathBuf {
        self.config_dir.join("mappings.toml")
    }

    pub fn include_gitconfig(&self) -> PathBuf {
        self.data_dir.join("include.gitconfig")
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.data_dir.join("profiles")
    }

    pub fn fragment(&self, profile: &str) -> PathBuf {
        self.profiles_dir().join(format!("{profile}.gitconfig"))
    }

    pub fn gh_dir(&self, profile: &str) -> PathBuf {
        self.data_dir.join("gh").join(profile)
    }

    /// Directory holding public keys materialised from the ssh-agent.
    pub fn ssh_pub_dir(&self) -> PathBuf {
        self.data_dir.join("ssh")
    }

    /// The derived public-key file for a profile whose key lives in the agent.
    pub fn ssh_pub(&self, profile: &str) -> PathBuf {
        self.ssh_pub_dir().join(format!("{profile}.pub"))
    }

    /// Machine-owned cache for the opportunistic update check (last-checked
    /// timestamp + newest version seen).
    pub fn update_state_json(&self) -> PathBuf {
        self.data_dir.join("update-check.json")
    }
}

/// Expand a leading `~` or `~/` in user-supplied input to an absolute path.
///
/// Only a leading tilde is expanded (git does not expand `~user`). A bare `~`
/// becomes `home`; `~/x` becomes `home/x`. Anything else is returned unchanged.
pub fn expand_tilde(input: &str, home: &Path) -> PathBuf {
    if input == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = input
        .strip_prefix("~/")
        .or_else(|| input.strip_prefix("~\\"))
    {
        return home.join(rest);
    }
    PathBuf::from(input)
}

/// Convert backslashes to forward slashes when operating in Windows style.
fn to_forward_slashes(path: &str, style: PathStyle) -> String {
    match style {
        PathStyle::Windows => path.replace('\\', "/"),
        PathStyle::Unix => path.to_string(),
    }
}

/// Normalise a directory path into the canonical string form gitid stores in
/// mappings: forward slashes and exactly one trailing slash.
pub fn normalize_dir(path: &str, style: PathStyle) -> String {
    let mut s = to_forward_slashes(path, style);
    while s.ends_with('/') {
        s.pop();
    }
    s.push('/');
    s
}

/// Replace a `home`-directory prefix with `~/`, keeping the result portable.
///
/// Both inputs are compared in forward-slash form. The home prefix only matches
/// on a path-segment boundary, so `/home/user2` is not rewritten when home is
/// `/home/user`.
pub fn contract_home(path: &str, home: &str, style: PathStyle) -> String {
    let path_fs = to_forward_slashes(path, style);
    let home_fs = to_forward_slashes(home, style)
        .trim_end_matches('/')
        .to_string();
    if home_fs.is_empty() {
        return path_fs;
    }
    if let Some(rest) = path_fs.strip_prefix(&home_fs) {
        if rest.is_empty() {
            return "~/".to_string();
        }
        if let Some(tail) = rest.strip_prefix('/') {
            return format!("~/{tail}");
        }
    }
    path_fs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_cases() {
        let home = Path::new("/home/jane");
        assert_eq!(expand_tilde("~", home), PathBuf::from("/home/jane"));
        assert_eq!(
            expand_tilde("~/.ssh/id", home),
            PathBuf::from("/home/jane/.ssh/id")
        );
        assert_eq!(expand_tilde("/abs/path", home), PathBuf::from("/abs/path"));
        assert_eq!(expand_tilde("relative", home), PathBuf::from("relative"));
        // ~user is not expanded.
        assert_eq!(expand_tilde("~bob/x", home), PathBuf::from("~bob/x"));
    }

    #[test]
    fn normalize_dir_trailing_slash() {
        assert_eq!(normalize_dir("/a/b", PathStyle::Unix), "/a/b/");
        assert_eq!(normalize_dir("/a/b/", PathStyle::Unix), "/a/b/");
        assert_eq!(normalize_dir("/a/b///", PathStyle::Unix), "/a/b/");
    }

    #[test]
    fn normalize_dir_windows_backslashes() {
        assert_eq!(
            normalize_dir("C:\\Users\\Jane\\code", PathStyle::Windows),
            "C:/Users/Jane/code/"
        );
        // Under Unix style backslashes are literal filename characters.
        assert_eq!(normalize_dir("a\\b", PathStyle::Unix), "a\\b/");
    }

    #[test]
    fn contract_home_segment_boundary() {
        assert_eq!(
            contract_home("/home/jane/code", "/home/jane", PathStyle::Unix),
            "~/code"
        );
        assert_eq!(
            contract_home("/home/jane", "/home/jane", PathStyle::Unix),
            "~/"
        );
        // Not a segment boundary: must not rewrite.
        assert_eq!(
            contract_home("/home/jane2/code", "/home/jane", PathStyle::Unix),
            "/home/jane2/code"
        );
        // Outside home: unchanged.
        assert_eq!(
            contract_home("/mnt/work", "/home/jane", PathStyle::Unix),
            "/mnt/work"
        );
    }

    #[test]
    fn contract_home_windows() {
        assert_eq!(
            contract_home(
                "C:\\Users\\Jane\\code",
                "C:\\Users\\Jane",
                PathStyle::Windows
            ),
            "~/code"
        );
    }
}
