//! `sync_all` — the single choke point every mutation flows through. It
//! regenerates all derived artefacts from the two sources of truth
//! (`profiles.toml`, `mappings.toml`) so the on-disk state is always a pure
//! function of them.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};

use crate::gitconfig::{ensure_include, render_fragment, render_include};
use crate::paths::{GitidPaths, PathStyle};
use crate::store::atomic_write;
use crate::store::mappings::MappingsFile;
use crate::store::profiles::{Profile, ProfilesFile};

/// Summary of what `sync_all` changed, for human-readable reporting.
#[derive(Debug, Default)]
pub struct SyncReport {
    pub fragments_written: Vec<String>,
    pub fragments_pruned: Vec<String>,
    pub gh_dirs_created: Vec<String>,
    pub include_changed: bool,
    pub mappings_changed: bool,
    pub global_include_added: bool,
    pub dangling_mappings: Vec<String>,
}

impl SyncReport {
    pub fn is_noop(&self) -> bool {
        self.fragments_written.is_empty()
            && self.fragments_pruned.is_empty()
            && self.gh_dirs_created.is_empty()
            && !self.include_changed
            && !self.mappings_changed
            && !self.global_include_added
    }
}

/// Compute the environment variables a profile contributes when active
/// (excluding `GITID_PROFILE`/`GITID_STATE`, which the activation layer adds).
/// Includes `GH_CONFIG_DIR` when gh isolation is enabled, plus the profile's
/// `[env]` table. Rejects values that cannot be safely exported.
pub fn activation_env(
    name: &str,
    profile: &Profile,
    paths: &GitidPaths,
) -> Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    if profile.gh_enabled() {
        let dir = paths.gh_dir(name);
        env.insert(
            "GH_CONFIG_DIR".to_string(),
            dir.to_string_lossy().into_owned(),
        );
    }
    for (k, v) in &profile.env {
        if v.contains('\n') || v.contains('\0') {
            bail!(
                "profile {name}: env var {k} contains a newline or NUL, which cannot be exported"
            );
        }
        env.insert(k.clone(), v.clone());
    }
    Ok(env)
}

/// Regenerate every derived artefact from the current stores.
pub fn sync_all(paths: &GitidPaths) -> Result<SyncReport> {
    let profiles = crate::store::profiles::load(&paths.profiles_toml())?;
    let mut mappings = MappingsFile::load(&paths.mappings_toml())?;
    let mut report = SyncReport::default();

    write_fragments(paths, &profiles, &mut report)?;
    prune_fragments(paths, &profiles, &mut report)?;
    refresh_mapping_env(paths, &profiles, &mut mappings, &mut report)?;
    write_include(paths, &mappings, &mut report)?;
    provision_gh_dirs(paths, &profiles, &mut report)?;

    report.global_include_added = ensure_include(&paths.home, &paths.include_gitconfig())?;

    Ok(report)
}

fn write_fragments(
    paths: &GitidPaths,
    profiles: &ProfilesFile,
    report: &mut SyncReport,
) -> Result<()> {
    for (name, profile) in &profiles.profiles {
        let fragment = render_fragment(profile, &paths.home);
        let path = paths.fragment(name);
        if file_differs(&path, &fragment)? {
            atomic_write(&path, &fragment)?;
            report.fragments_written.push(name.clone());
        }
    }
    Ok(())
}

fn prune_fragments(
    paths: &GitidPaths,
    profiles: &ProfilesFile,
    report: &mut SyncReport,
) -> Result<()> {
    let dir = paths.profiles_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("could not read {}", dir.display())),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("gitconfig") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if !profiles.profiles.contains_key(stem) {
            std::fs::remove_file(&path)
                .with_context(|| format!("could not remove {}", path.display()))?;
            report.fragments_pruned.push(stem.to_string());
        }
    }
    Ok(())
}

fn refresh_mapping_env(
    paths: &GitidPaths,
    profiles: &ProfilesFile,
    mappings: &mut MappingsFile,
    report: &mut SyncReport,
) -> Result<()> {
    let before = mappings.clone_for_compare();
    let mut dangling = BTreeSet::new();
    for mapping in &mut mappings.mappings {
        match profiles.profiles.get(&mapping.profile) {
            Some(profile) => {
                mapping.env = activation_env(&mapping.profile, profile, paths)?;
            }
            None => {
                mapping.env = BTreeMap::new();
                dangling.insert(mapping.profile.clone());
            }
        }
    }
    mappings.sort();
    if mappings.clone_for_compare() != before {
        mappings.save(&paths.mappings_toml())?;
        report.mappings_changed = true;
    }
    report.dangling_mappings = dangling.into_iter().collect();
    Ok(())
}

fn write_include(
    paths: &GitidPaths,
    mappings: &MappingsFile,
    report: &mut SyncReport,
) -> Result<()> {
    let include = render_include(
        &mappings.mappings,
        &paths.home.to_string_lossy(),
        PathStyle::host(),
    );
    let path = paths.include_gitconfig();
    if file_differs(&path, &include)? {
        atomic_write(&path, &include)?;
        report.include_changed = true;
    }
    Ok(())
}

fn provision_gh_dirs(
    paths: &GitidPaths,
    profiles: &ProfilesFile,
    report: &mut SyncReport,
) -> Result<()> {
    for (name, profile) in &profiles.profiles {
        if profile.gh_enabled() {
            let dir = paths.gh_dir(name);
            if !dir.exists() {
                std::fs::create_dir_all(&dir)
                    .with_context(|| format!("could not create {}", dir.display()))?;
                report.gh_dirs_created.push(name.clone());
            }
        }
    }
    Ok(())
}

/// Whether the file at `path` differs from `contents` (missing counts as
/// differing). Lets sync report only real changes and keep writes idempotent.
fn file_differs(path: &std::path::Path, contents: &str) -> Result<bool> {
    match std::fs::read_to_string(path) {
        Ok(existing) => Ok(existing != contents),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(e).with_context(|| format!("could not read {}", path.display())),
    }
}

impl MappingsFile {
    /// Clone the comparable content (ignoring nothing currently, but isolated so
    /// the comparison is explicit).
    fn clone_for_compare(&self) -> Vec<crate::store::mappings::Mapping> {
        self.mappings.clone()
    }
}
