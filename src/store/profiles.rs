//! `profiles.toml` — the hand-editable source of truth describing each named
//! identity. Mutations go through a [`toml_edit::DocumentMut`] so user comments
//! and key ordering survive programmatic edits.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::store::atomic_write;

/// The current `profiles.toml` schema version.
pub const VERSION: u32 = 1;

/// Typed view of `profiles.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesFile {
    pub version: u32,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

impl Default for ProfilesFile {
    fn default() -> Self {
        Self {
            version: VERSION,
            profiles: BTreeMap::new(),
        }
    }
}

/// A single identity. The profile's name is the map key in [`ProfilesFile`],
/// not a field here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    /// git `user.name`.
    pub name: String,
    /// git `user.email`.
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh: Option<Ssh>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing: Option<Signing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gh: Option<Gh>,
    /// Extra environment variables exported by the shell hook when active.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Raw `git config` keys written verbatim into the profile fragment.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ssh {
    /// Path to the private key (may contain a leading `~`).
    pub key: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SigningFormat {
    Ssh,
    Openpgp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signing {
    pub format: SigningFormat,
    /// For `ssh`: path to the public key. For `openpgp`: the key id.
    pub key: String,
    #[serde(default)]
    pub commits: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<bool>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gh {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for Gh {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Profile {
    /// Whether this profile provisions an isolated `GH_CONFIG_DIR`.
    pub fn gh_enabled(&self) -> bool {
        self.gh.as_ref().is_some_and(|g| g.enabled)
    }
}

/// Validate a profile name. Names become filenames and gitconfig keys, so they
/// are restricted to a conservative slug.
pub fn validate_name(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !ok {
        bail!(
            "invalid profile name {name:?}: use lowercase letters, digits, '-' and '_', \
             starting with a letter or digit"
        );
    }
    Ok(())
}

/// Load and type-check `profiles.toml`. A missing file yields an empty default.
pub fn load(path: &Path) -> Result<ProfilesFile> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ProfilesFile::default()),
        Err(e) => {
            return Err(e).with_context(|| format!("could not read {}", path.display()));
        }
    };
    let parsed: ProfilesFile = toml_edit::de::from_str(&text)
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

/// Load the editable document, preserving comments and formatting. A missing
/// file yields a fresh document with just the version header.
pub fn load_doc(path: &Path) -> Result<DocumentMut> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(new_document());
        }
        Err(e) => {
            return Err(e).with_context(|| format!("could not read {}", path.display()));
        }
    };
    text.parse::<DocumentMut>()
        .with_context(|| format!("could not parse {}", path.display()))
}

fn new_document() -> DocumentMut {
    let mut doc = DocumentMut::new();
    doc["version"] = toml_edit::value(VERSION as i64);
    doc.as_table_mut()
        .decor_mut()
        .set_prefix("# gitid profiles — edit freely; run `gitid sync` afterwards.\n");
    doc
}

/// Insert or replace a profile in the document, preserving everything else.
pub fn upsert_profile(doc: &mut DocumentMut, name: &str, profile: &Profile) -> Result<()> {
    let fragment = toml_edit::ser::to_document(profile).context("could not serialise profile")?;
    let mut table = fragment.as_table().clone();
    table.set_implicit(false);

    let profiles = doc["profiles"].or_insert(toml_edit::Item::Table({
        let mut t = toml_edit::Table::new();
        t.set_implicit(true);
        t
    }));
    profiles[name] = toml_edit::Item::Table(table);
    Ok(())
}

/// Remove a profile from the document. Returns whether it existed.
pub fn remove_profile(doc: &mut DocumentMut, name: &str) -> bool {
    doc.get_mut("profiles")
        .and_then(|p| p.as_table_mut())
        .and_then(|t| t.remove(name))
        .is_some()
}

/// Persist the document atomically.
pub fn save_doc(path: &Path, doc: &DocumentMut) -> Result<()> {
    atomic_write(path, &doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Profile {
        Profile {
            name: "Jane Doe".into(),
            email: "jane@corp.example".into(),
            ssh: Some(Ssh {
                key: "~/.ssh/id_ed25519_work".into(),
            }),
            signing: Some(Signing {
                format: SigningFormat::Ssh,
                key: "~/.ssh/id_ed25519_work.pub".into(),
                commits: true,
                tags: None,
            }),
            gh: Some(Gh { enabled: true }),
            env: BTreeMap::new(),
            extra: BTreeMap::new(),
        }
    }

    #[test]
    fn name_validation() {
        assert!(validate_name("work").is_ok());
        assert!(validate_name("client-acme").is_ok());
        assert!(validate_name("oss_2").is_ok());
        assert!(validate_name("2fa").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("-bad").is_err());
        assert!(validate_name("Work").is_err());
        assert!(validate_name("has space").is_err());
        assert!(validate_name("dots.bad").is_err());
    }

    #[test]
    fn roundtrip_preserves_comments() {
        let src = "\
# top comment
version = 1

[profiles.personal]
name = \"Pat\"  # inline
email = \"pat@example.com\"
";
        let mut doc: DocumentMut = src.parse().unwrap();
        upsert_profile(&mut doc, "work", &sample()).unwrap();
        let out = doc.to_string();
        assert!(out.contains("# top comment"));
        assert!(out.contains("# inline"));
        assert!(out.contains("[profiles.work]"));
        assert!(out.contains("[profiles.personal]"));

        // And it must round-trip back through the typed loader.
        let parsed: ProfilesFile = toml_edit::de::from_str(&out).unwrap();
        assert_eq!(parsed.profiles.len(), 2);
        assert_eq!(parsed.profiles["work"].name, "Jane Doe");
        assert_eq!(parsed.profiles["work"], sample());
    }

    #[test]
    fn remove_works() {
        let mut doc = new_document();
        upsert_profile(&mut doc, "work", &sample()).unwrap();
        assert!(remove_profile(&mut doc, "work"));
        assert!(!remove_profile(&mut doc, "work"));
    }

    #[test]
    fn new_document_has_header() {
        let doc = new_document();
        let s = doc.to_string();
        assert!(s.contains("# gitid profiles"));
        assert!(s.contains("version = 1"));
    }
}
