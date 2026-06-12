//! `mappings.toml` — machine-owned directory→profile assignments, plus a
//! derived `env` cache per mapping so the hot `gitid env` path reads exactly one
//! small file.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::paths::{PathStyle, normalize_dir};
use crate::store::atomic_write;

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingsFile {
    pub version: u32,
    #[serde(default, rename = "mapping")]
    pub mappings: Vec<Mapping>,
}

impl Default for MappingsFile {
    fn default() -> Self {
        Self {
            version: VERSION,
            mappings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Mapping {
    /// Canonical directory: forward slashes, single trailing slash.
    pub dir: String,
    /// The literal user-typed form, when it differed from `dir` (e.g. a
    /// symlinked tree). Used to emit a second `includeIf` line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir_literal: Option<String>,
    pub profile: String,
    #[serde(default)]
    pub case_insensitive: bool,
    /// Derived at sync time; read verbatim by `gitid env`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

impl MappingsFile {
    pub fn load(path: &Path) -> Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => {
                return Err(e).with_context(|| format!("could not read {}", path.display()));
            }
        };
        let parsed: MappingsFile = toml_edit::de::from_str(&text)
            .with_context(|| format!("could not parse {}", path.display()))?;
        if parsed.version != VERSION {
            bail!(
                "{}: unsupported version {} (expected {VERSION})",
                path.display(),
                parsed.version
            );
        }
        Ok(parsed)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text =
            toml_edit::ser::to_string_pretty(self).context("could not serialise mappings")?;
        atomic_write(path, &text)
    }

    /// Insert or replace the mapping for `dir` (compared canonically).
    pub fn upsert(&mut self, mapping: Mapping) {
        if let Some(existing) = self.mappings.iter_mut().find(|m| m.dir == mapping.dir) {
            *existing = mapping;
        } else {
            self.mappings.push(mapping);
        }
        self.sort();
    }

    /// Remove the mapping for the canonical `dir`. Returns whether one existed.
    pub fn remove_dir(&mut self, dir: &str) -> bool {
        let before = self.mappings.len();
        self.mappings.retain(|m| m.dir != dir);
        self.mappings.len() != before
    }

    /// Order by ascending directory length so that, when rendered into
    /// `include.gitconfig`, more specific (longer) directories come last and win
    /// under gitconfig's last-wins precedence.
    pub fn sort(&mut self) {
        self.mappings
            .sort_by(|a, b| a.dir.len().cmp(&b.dir.len()).then(a.dir.cmp(&b.dir)));
    }
}

/// Longest-prefix match of `cwd` against the mappings.
///
/// `cwd` is normalised to the canonical trailing-slash form before comparison.
/// Case-insensitive mappings are matched ASCII-case-insensitively. When several
/// mappings match, the one with the longest `dir` wins (the most specific tree).
pub fn match_dir<'a>(mappings: &'a [Mapping], cwd: &Path, style: PathStyle) -> Option<&'a Mapping> {
    let cwd_norm = normalize_dir(&cwd.to_string_lossy(), style);
    mappings
        .iter()
        .filter(|m| prefix_matches(&m.dir, &cwd_norm, m.case_insensitive))
        .max_by_key(|m| m.dir.len())
}

/// Whether `dir` (canonical, trailing slash) is a prefix of `cwd` (canonical,
/// trailing slash) on a path-segment boundary. Because both end in `/`, a plain
/// `starts_with` already respects segment boundaries.
fn prefix_matches(dir: &str, cwd: &str, icase: bool) -> bool {
    if icase {
        let cwd_lower = cwd.to_ascii_lowercase();
        let dir_lower = dir.to_ascii_lowercase();
        cwd_lower.starts_with(&dir_lower)
    } else {
        cwd.starts_with(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn m(dir: &str, profile: &str) -> Mapping {
        Mapping {
            dir: normalize_dir(dir, PathStyle::Unix),
            dir_literal: None,
            profile: profile.into(),
            case_insensitive: false,
            env: BTreeMap::new(),
        }
    }

    #[test]
    fn longest_prefix_wins() {
        let maps = vec![
            m("/home/u/code", "personal"),
            m("/home/u/code/work", "work"),
        ];
        let cwd = PathBuf::from("/home/u/code/work/repo");
        assert_eq!(
            match_dir(&maps, &cwd, PathStyle::Unix).unwrap().profile,
            "work"
        );
        let cwd2 = PathBuf::from("/home/u/code/oss/repo");
        assert_eq!(
            match_dir(&maps, &cwd2, PathStyle::Unix).unwrap().profile,
            "personal"
        );
    }

    #[test]
    fn no_match_returns_none() {
        let maps = vec![m("/home/u/work", "work")];
        let cwd = PathBuf::from("/tmp/elsewhere");
        assert!(match_dir(&maps, &cwd, PathStyle::Unix).is_none());
    }

    #[test]
    fn exact_dir_matches() {
        let maps = vec![m("/home/u/work", "work")];
        let cwd = PathBuf::from("/home/u/work");
        assert_eq!(
            match_dir(&maps, &cwd, PathStyle::Unix).unwrap().profile,
            "work"
        );
    }

    #[test]
    fn segment_boundary_not_fooled_by_prefix() {
        // /home/u/work must not match /home/u/work2
        let maps = vec![m("/home/u/work", "work")];
        let cwd = PathBuf::from("/home/u/work2/repo");
        assert!(match_dir(&maps, &cwd, PathStyle::Unix).is_none());
    }

    #[test]
    fn case_insensitive_matching() {
        let mut mp = m("/Users/Jane/Code", "work");
        mp.case_insensitive = true;
        let maps = vec![mp];
        let cwd = PathBuf::from("/users/jane/code/repo");
        assert_eq!(
            match_dir(&maps, &cwd, PathStyle::Unix).unwrap().profile,
            "work"
        );
    }

    #[test]
    fn case_sensitive_does_not_match_different_case() {
        let maps = vec![m("/Users/Jane/Code", "work")];
        let cwd = PathBuf::from("/users/jane/code/repo");
        assert!(match_dir(&maps, &cwd, PathStyle::Unix).is_none());
    }

    #[test]
    fn sort_orders_by_length() {
        let mut f = MappingsFile::default();
        f.upsert(m("/home/u/code/work", "work"));
        f.upsert(m("/home/u/code", "personal"));
        assert_eq!(f.mappings[0].profile, "personal");
        assert_eq!(f.mappings[1].profile, "work");
    }

    #[test]
    fn upsert_replaces_same_dir() {
        let mut f = MappingsFile::default();
        f.upsert(m("/home/u/work", "old"));
        f.upsert(m("/home/u/work", "new"));
        assert_eq!(f.mappings.len(), 1);
        assert_eq!(f.mappings[0].profile, "new");
    }
}
