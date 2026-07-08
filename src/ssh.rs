//! Discovery of SSH private keys for the `add` wizard.
//!
//! Unlike a filename-prefix match, this inspects file *contents* so any private
//! key is found regardless of how it is named, and additionally pulls in paths
//! referenced by `IdentityFile` directives in `~/.ssh/config`.

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use crate::paths::{PathStyle, contract_home, expand_tilde};

/// A discovered SSH private key: the stored path (tilde-contracted for display),
/// plus a best-effort algorithm and comment for the picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshKey {
    /// Path as it should be stored/used (e.g. `~/.ssh/id_ed25519`).
    pub path: String,
    /// Detected algorithm: `ed25519`, `rsa`, `ecdsa`, `dsa`, or a header hint.
    pub kind: String,
    /// Trailing comment from the sibling public key, if any.
    pub comment: Option<String>,
}

impl fmt::Display for SshKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.comment {
            Some(c) => write!(f, "{}  ({}, {})", self.path, self.kind, c),
            None => write!(f, "{}  ({})", self.path, self.kind),
        }
    }
}

/// Discover candidate private keys under `~/.ssh` (by content) plus any
/// `IdentityFile` paths named in `~/.ssh/config`. Results are sorted and deduped.
pub fn discover(home: &Path) -> Vec<SshKey> {
    let ssh_dir = home.join(".ssh");
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut keys: Vec<SshKey> = Vec::new();

    // 1. Content scan of ~/.ssh.
    if let Ok(entries) = std::fs::read_dir(&ssh_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_ignored(&name) {
                continue;
            }
            if let Some(kind) = private_key_kind(&path) {
                push_key(home, &path, Some(kind), &mut seen, &mut keys);
            }
        }
    }

    // 2. IdentityFile paths from ~/.ssh/config (may live outside ~/.ssh).
    for raw in identity_files(&ssh_dir.join("config")) {
        let abs = expand_tilde(&raw, home);
        if !abs.is_file() {
            continue;
        }
        let kind = private_key_kind(&abs);
        push_key(home, &abs, kind, &mut seen, &mut keys);
    }

    keys.sort_by(|a, b| a.path.cmp(&b.path));
    keys
}

fn push_key(
    home: &Path,
    path: &Path,
    kind: Option<&'static str>,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<SshKey>,
) {
    // Dedupe on the canonical path where possible, else the raw path.
    let canon = std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned());
    if !seen.insert(canon) {
        return;
    }
    let display = contract_home(
        &path.to_string_lossy(),
        &home.to_string_lossy(),
        PathStyle::host(),
    );
    let (pub_kind, comment) = pub_metadata(path);
    let kind = pub_kind
        .or_else(|| kind.map(str::to_string))
        .unwrap_or_else(|| "key".to_string());
    out.push(SshKey {
        path: display,
        kind,
        comment,
    });
}

/// Files in ~/.ssh that are never private keys.
fn is_ignored(name: &str) -> bool {
    name.ends_with(".pub")
        || name == "config"
        || name == "authorized_keys"
        || name == "authorized_keys2"
        || name.starts_with("known_hosts")
        || name.starts_with('.')
}

/// Peek at a file's first bytes and classify it as a private key by header.
fn private_key_kind(path: &Path) -> Option<&'static str> {
    use std::io::Read;
    let mut buf = [0u8; 200];
    let mut f = std::fs::File::open(path).ok()?;
    let n = f.read(&mut buf).ok()?;
    let head = String::from_utf8_lossy(&buf[..n]);
    let first = head.lines().next().unwrap_or("").trim();
    let marker = |m: &str| head.contains(m);
    if marker("BEGIN OPENSSH PRIVATE KEY") {
        Some("openssh")
    } else if marker("BEGIN RSA PRIVATE KEY") {
        Some("rsa")
    } else if marker("BEGIN DSA PRIVATE KEY") {
        Some("dsa")
    } else if marker("BEGIN EC PRIVATE KEY") {
        Some("ecdsa")
    } else if marker("BEGIN ENCRYPTED PRIVATE KEY") || marker("BEGIN PRIVATE KEY") {
        Some("pkcs8")
    } else if first.starts_with("PuTTY-User-Key-File-") {
        Some("putty")
    } else {
        None
    }
}

/// Read algorithm + comment from a sibling `<key>.pub`, if present.
fn pub_metadata(private: &Path) -> (Option<String>, Option<String>) {
    let pub_path = {
        let mut s = private.as_os_str().to_os_string();
        s.push(".pub");
        std::path::PathBuf::from(s)
    };
    let Ok(line) = std::fs::read_to_string(&pub_path) else {
        return (None, None);
    };
    let mut parts = line.split_whitespace();
    let algo = parts.next().map(normalize_algo);
    // The comment is everything after the base64 blob.
    let _blob = parts.next();
    let comment: String = parts.collect::<Vec<_>>().join(" ");
    let comment = (!comment.is_empty()).then_some(comment);
    (algo, comment)
}

fn normalize_algo(token: &str) -> String {
    match token {
        "ssh-ed25519" => "ed25519".to_string(),
        "ssh-rsa" => "rsa".to_string(),
        "ssh-dss" => "dsa".to_string(),
        t if t.starts_with("ecdsa-sha2-") => "ecdsa".to_string(),
        t if t.starts_with("sk-ssh-ed25519") => "ed25519-sk".to_string(),
        t if t.starts_with("sk-ecdsa-sha2-") => "ecdsa-sk".to_string(),
        other => other.to_string(),
    }
}

/// Extract `IdentityFile` values from an ssh config file (best effort, ignoring
/// Host scoping — any referenced key is a reasonable candidate to offer).
fn identity_files(config: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(config) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Split on the first run of whitespace or '='.
        let mut it = line.splitn(2, |c: char| c.is_whitespace() || c == '=');
        let key = it.next().unwrap_or("").trim();
        if !key.eq_ignore_ascii_case("identityfile") {
            continue;
        }
        if let Some(val) = it.next() {
            let val = val.trim().trim_matches('"');
            if !val.is_empty() {
                out.push(val.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn discovers_keys_by_content_not_name() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let ssh = home.join(".ssh");

        // A private key with a non-`id_` name is still found by header.
        write(
            &ssh.join("work"),
            "-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n",
        );
        write(
            &ssh.join("work.pub"),
            "ssh-ed25519 AAAAC3Nz jane@corp.example\n",
        );
        // Legacy PEM RSA key.
        write(
            &ssh.join("legacy.pem"),
            "-----BEGIN RSA PRIVATE KEY-----\nxyz\n",
        );
        // Non-keys that must be ignored.
        write(&ssh.join("known_hosts"), "github.com ssh-ed25519 AAAA\n");
        write(&ssh.join("config"), "Host *\n  IdentityFile ~/keys/ext\n");
        write(&ssh.join("id_ed25519.pub"), "ssh-ed25519 AAAA orphan\n");

        // IdentityFile target outside ~/.ssh.
        write(
            &home.join("keys/ext"),
            "-----BEGIN EC PRIVATE KEY-----\nq\n",
        );

        let keys = discover(home);
        let by_path: std::collections::BTreeMap<_, _> =
            keys.iter().map(|k| (k.path.as_str(), k)).collect();

        // Found the non-id_ key with algo+comment from its .pub.
        let work = by_path.get("~/.ssh/work").expect("work key found");
        assert_eq!(work.kind, "ed25519");
        assert_eq!(work.comment.as_deref(), Some("jane@corp.example"));

        // Found the PEM key (kind from header, no .pub).
        assert_eq!(by_path.get("~/.ssh/legacy.pem").unwrap().kind, "rsa");

        // Found the ssh_config IdentityFile target outside ~/.ssh.
        assert_eq!(by_path.get("~/keys/ext").unwrap().kind, "ecdsa");

        // Ignored non-keys and orphan .pub.
        assert!(!by_path.contains_key("~/.ssh/known_hosts"));
        assert!(!by_path.contains_key("~/.ssh/config"));
        assert!(!by_path.contains_key("~/.ssh/id_ed25519"));
    }
}
