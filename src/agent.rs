//! ssh-agent connectivity and selector matching. Only [`endpoint`] and
//! [`list_keys`] touch the environment/socket; everything else is pure so it
//! can be unit-tested without a running agent.

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use ssh_agent_client_rs::{Client, Identity};
use ssh_key::{HashAlg, PublicKey};

/// The named pipe of the OpenSSH agent shipped with Windows.
#[cfg(windows)]
const DEFAULT_WINDOWS_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";

/// A public key held by the ssh-agent.
#[derive(Debug, Clone)]
pub struct AgentKey {
    pub public: PublicKey,
}

impl AgentKey {
    /// The single-line authorized_keys form: `ssh-ed25519 AAAA… comment`.
    pub fn line(&self) -> String {
        self.public
            .to_openssh()
            .unwrap_or_else(|_| String::new())
            .trim_end()
            .to_string()
    }

    /// The `SHA256:…` fingerprint.
    pub fn fingerprint(&self) -> String {
        self.public.fingerprint(HashAlg::Sha256).to_string()
    }

    /// Human-readable picker/display label: algorithm, comment, fingerprint.
    pub fn label(&self) -> String {
        let comment = self.public.comment();
        if comment.is_empty() {
            format!("{} {}", self.public.algorithm(), self.fingerprint())
        } else {
            format!(
                "{} {} ({})",
                self.public.algorithm(),
                comment,
                self.fingerprint()
            )
        }
    }
}

/// Resolve the agent endpoint from a `SSH_AUTH_SOCK` value. On unix the
/// variable is required; on Windows the OpenSSH agent's named pipe is the
/// default when it is unset.
pub fn endpoint_from(sock: Option<OsString>) -> Result<PathBuf> {
    match sock {
        Some(s) if !s.is_empty() => Ok(PathBuf::from(s)),
        _ => {
            #[cfg(windows)]
            {
                Ok(PathBuf::from(DEFAULT_WINDOWS_PIPE))
            }
            #[cfg(not(windows))]
            {
                bail!("SSH_AUTH_SOCK is not set; is an ssh-agent running?")
            }
        }
    }
}

/// The agent endpoint for this process's environment.
pub fn endpoint() -> Result<PathBuf> {
    endpoint_from(std::env::var_os("SSH_AUTH_SOCK"))
}

/// Connect to the agent and list its public keys. Certificates are skipped.
pub fn list_keys() -> Result<Vec<AgentKey>> {
    let endpoint = endpoint()?;
    let mut client = Client::connect(&endpoint).with_context(|| {
        format!(
            "could not connect to the ssh-agent at {}",
            endpoint.display()
        )
    })?;
    let identities = client
        .list_all_identities()
        .with_context(|| format!("could not list ssh-agent keys at {}", endpoint.display()))?;
    Ok(identities
        .into_iter()
        .filter_map(|i| match i {
            Identity::PublicKey(pk) => Some(AgentKey {
                public: pk.into_owned(),
            }),
            Identity::Certificate(_) => None,
        })
        .collect())
}

/// Check that `selector` resolves to exactly one key in the running agent.
/// Used at add/edit time so the sync that follows cannot fail.
pub fn validate_selector(selector: &str) -> Result<()> {
    let keys = list_keys()?;
    select(&keys, selector)?;
    Ok(())
}

/// Whether `selector` matches an agent key: a `SHA256:` fingerprint (prefix
/// match allowed) or a comment (exact, else unique substring).
fn matches(key: &AgentKey, selector: &str) -> bool {
    if selector.starts_with("SHA256:") {
        key.fingerprint().starts_with(selector)
    } else {
        key.public.comment() == selector
    }
}

/// Resolve a selector against the agent's keys.
///
/// `SHA256:` selectors match on fingerprint prefix; anything else matches an
/// exact comment first, falling back to a unique comment substring. Ambiguity
/// and no-match are errors that list what the agent holds.
pub fn select<'a>(keys: &'a [AgentKey], selector: &str) -> Result<&'a AgentKey> {
    if selector.trim().is_empty() {
        bail!("the ssh-agent key selector is empty; pass a SHA256: fingerprint or a key comment");
    }
    let exact: Vec<&AgentKey> = keys.iter().filter(|k| matches(k, selector)).collect();
    let candidates = if exact.is_empty() && !selector.starts_with("SHA256:") {
        keys.iter()
            .filter(|k| k.public.comment().contains(selector))
            .collect()
    } else {
        exact
    };
    match candidates.as_slice() {
        [one] => Ok(one),
        [] => bail!(
            "no ssh-agent key matches {selector:?}; the agent holds:\n{}",
            describe(keys)
        ),
        many => bail!(
            "{selector:?} is ambiguous; it matches:\n{}",
            describe_refs(many)
        ),
    }
}

fn describe(keys: &[AgentKey]) -> String {
    if keys.is_empty() {
        return "  (no keys)".to_string();
    }
    keys.iter()
        .map(|k| format!("  {}", k.label()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn describe_refs(keys: &[&AgentKey]) -> String {
    keys.iter()
        .map(|k| format!("  {}", k.label()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // A real (throwaway) ed25519 public key for deterministic tests.
    const KEY_A: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB5RCT+eqSGqnokTIWWpaBW4uUC1MqNCTBddYzZSN99a jane@corp.example";
    const KEY_B: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPLix5wYCbrjbUujcCU9kSCsO0oNsiCn5AWaVI6RTOWG pat@corp.example";

    fn keys() -> Vec<AgentKey> {
        [KEY_A, KEY_B]
            .iter()
            .map(|l| AgentKey {
                public: PublicKey::from_openssh(l).unwrap(),
            })
            .collect()
    }

    #[test]
    fn line_round_trips() {
        let keys = keys();
        assert_eq!(keys[0].line(), KEY_A);
    }

    #[test]
    fn select_by_exact_comment() {
        let keys = keys();
        let k = select(&keys, "jane@corp.example").unwrap();
        assert_eq!(k.line(), KEY_A);
    }

    #[test]
    fn select_by_comment_substring() {
        let keys = keys();
        let k = select(&keys, "pat@").unwrap();
        assert_eq!(k.line(), KEY_B);
    }

    #[test]
    fn select_by_fingerprint_prefix() {
        let keys = keys();
        let fp = keys[1].fingerprint();
        let k = select(&keys, &fp[..14]).unwrap();
        assert_eq!(k.line(), KEY_B);
        // Full fingerprint too.
        assert_eq!(select(&keys, &fp).unwrap().line(), KEY_B);
    }

    #[test]
    fn select_ambiguous_substring_errors() {
        let keys = keys();
        let err = select(&keys, "corp.example").unwrap_err().to_string();
        assert!(err.contains("ambiguous"), "{err}");
        assert!(err.contains("jane@corp.example"), "{err}");
    }

    #[test]
    fn select_no_match_lists_keys() {
        let keys = keys();
        let err = select(&keys, "nobody").unwrap_err().to_string();
        assert!(err.contains("no ssh-agent key matches"), "{err}");
        assert!(err.contains("pat@corp.example"), "{err}");
    }

    #[test]
    fn select_empty_agent() {
        let err = select(&[], "x").unwrap_err().to_string();
        assert!(err.contains("(no keys)"), "{err}");
    }

    #[test]
    fn select_empty_selector_is_rejected() {
        let keys = keys();
        let err = select(&keys, "  ").unwrap_err().to_string();
        assert!(err.contains("selector is empty"), "{err}");
    }

    #[cfg(not(windows))]
    #[test]
    fn endpoint_requires_sock_on_unix() {
        assert!(endpoint_from(None).is_err());
        assert!(endpoint_from(Some(OsString::new())).is_err());
        assert_eq!(
            endpoint_from(Some("/tmp/agent.sock".into())).unwrap(),
            PathBuf::from("/tmp/agent.sock")
        );
    }

    #[cfg(windows)]
    #[test]
    fn endpoint_defaults_to_pipe_on_windows() {
        assert_eq!(
            endpoint_from(None).unwrap(),
            PathBuf::from(DEFAULT_WINDOWS_PIPE)
        );
        assert_eq!(
            endpoint_from(Some(r"\\.\pipe\custom".into())).unwrap(),
            PathBuf::from(r"\\.\pipe\custom")
        );
    }
}
