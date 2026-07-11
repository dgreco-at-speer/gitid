//! Shared test harness: an isolated HOME + config/data dirs in a tempdir, with
//! `GitidPaths` constructed directly so library-level tests never mutate the
//! process environment (and therefore stay parallel-safe).

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use gitid::paths::GitidPaths;
use gitid::store::profiles::{self, Profile};
use tempfile::TempDir;
use toml_edit::DocumentMut;

pub struct TestEnv {
    pub tmp: TempDir,
    pub paths: GitidPaths,
}

impl TestEnv {
    pub fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let paths = GitidPaths {
            config_dir: home.join(".config").join("gitid"),
            data_dir: home.join(".local").join("share").join("gitid"),
            home,
        };
        Self { tmp, paths }
    }

    pub fn home(&self) -> &Path {
        &self.paths.home
    }

    /// An `assert_cmd` invocation of the `gitid` binary with a fully isolated
    /// environment: tempdir HOME, gitid config/data dirs, no system git config,
    /// and the global gitconfig pinned to HOME/.gitconfig.
    pub fn gitid(&self) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("gitid").unwrap();
        cmd.env("HOME", &self.paths.home)
            .env("GITID_CONFIG_DIR", &self.paths.config_dir)
            .env("GITID_DATA_DIR", &self.paths.data_dir)
            .env("GIT_CONFIG_GLOBAL", self.global_gitconfig())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("XDG_DATA_HOME")
            // The developer's own shell may have an active gitid profile and a
            // running ssh-agent; neither may leak into assertions.
            .env_remove("GITID_PROFILE")
            .env_remove("GITID_STATE")
            .env_remove("GH_CONFIG_DIR")
            .env_remove("SSH_AUTH_SOCK");
        cmd
    }

    pub fn global_gitconfig(&self) -> PathBuf {
        self.paths.home.join(".gitconfig")
    }

    /// Add (or replace) a profile in `profiles.toml`, preserving the rest.
    pub fn add_profile(&self, name: &str, profile: &Profile) {
        let path = self.paths.profiles_toml();
        let mut doc: DocumentMut = profiles::load_doc(&path).unwrap();
        profiles::upsert_profile(&mut doc, name, profile).unwrap();
        profiles::save_doc(&path, &doc).unwrap();
    }

    pub fn write_global_gitconfig(&self, contents: &str) {
        std::fs::write(self.global_gitconfig(), contents).unwrap();
    }

    pub fn read_global_gitconfig(&self) -> String {
        std::fs::read_to_string(self.global_gitconfig()).unwrap()
    }

    /// `git -C <dir> config --get <key>` against this isolated HOME.
    pub fn git_config_in(&self, dir: &Path, key: &str) -> Option<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["config", "--get", key])
            .env("HOME", &self.paths.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_CONFIG_GLOBAL")
            .env_remove("XDG_CONFIG_HOME")
            .output()
            .unwrap();
        if out.status.success() {
            Some(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
        } else {
            None
        }
    }

    /// `git init <dir>` with this isolated HOME.
    pub fn git_init(&self, dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        let out = Command::new("git")
            .arg("init")
            .arg("-q")
            .arg(dir)
            .env("HOME", &self.paths.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "git init failed");
    }
}

/// A simple full-featured profile for tests.
pub fn sample_profile(name: &str, email: &str) -> Profile {
    Profile {
        name: name.into(),
        email: email.into(),
        ssh: None,
        signing: None,
        gh: None,
        env: Default::default(),
        extra: Default::default(),
    }
}
