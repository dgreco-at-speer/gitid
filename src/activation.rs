//! The pure activation diff: given the profile a directory wants, the
//! `GITID_STATE` carried in the environment, and the current values of managed
//! variables, compute the set of env changes to emit. No I/O.

use std::collections::BTreeMap;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

/// The variable that always reflects the active profile (for prompts).
pub const PROFILE_VAR: &str = "GITID_PROFILE";
/// The internal var carrying the saved-values diff so deactivation can restore.
pub const STATE_VAR: &str = "GITID_STATE";

const STATE_VERSION: u32 = 1;

/// A single environment mutation to render per shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvOp {
    Set(String, String),
    Unset(String),
}

/// What a directory's profile wants active (excluding `GITID_PROFILE`/`GITID_STATE`).
#[derive(Debug, Clone)]
pub struct Target {
    pub profile: String,
    pub env: BTreeMap<String, String>,
}

/// Decoded `GITID_STATE`: the currently-active profile and the pre-activation
/// values of every variable we set (`None` = the variable was unset).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct State {
    pub v: u32,
    pub profile: String,
    pub saved: BTreeMap<String, Option<String>>,
}

impl State {
    pub fn encode(&self) -> String {
        let json = serde_json::to_vec(self).expect("state serialises");
        URL_SAFE_NO_PAD.encode(json)
    }

    /// Decode `GITID_STATE`. Returns `None` (treated as "no active profile") on
    /// any decode/version error rather than guessing.
    pub fn decode(raw: &str) -> Option<State> {
        let bytes = URL_SAFE_NO_PAD.decode(raw.trim()).ok()?;
        let state: State = serde_json::from_slice(&bytes).ok()?;
        if state.v != STATE_VERSION {
            return None;
        }
        Some(state)
    }
}

/// Compute the transition from the current state to the target profile.
///
/// `current_env` reports the *inherited* value of a variable. Returns an empty
/// vec when nothing changes (the common case: same profile, or none↔none).
pub fn compute_transition(
    target: Option<&Target>,
    state: Option<State>,
    current_env: &dyn Fn(&str) -> Option<String>,
) -> Vec<EnvOp> {
    let target_profile = target.map(|t| t.profile.as_str());
    let state_profile = state.as_ref().map(|s| s.profile.as_str());
    if target_profile == state_profile {
        return Vec::new();
    }

    let mut ops = Vec::new();

    // The variables the new profile will set (so we needn't restore-then-reset).
    let will_set: std::collections::BTreeSet<&str> = match target {
        Some(t) => std::iter::once(PROFILE_VAR)
            .chain(t.env.keys().map(String::as_str))
            .collect(),
        None => std::collections::BTreeSet::new(),
    };

    // Deactivate the old profile: restore saved variables not about to be reset.
    if let Some(state) = &state {
        for (name, prev) in &state.saved {
            if will_set.contains(name.as_str()) {
                continue;
            }
            match prev {
                Some(v) => ops.push(EnvOp::Set(name.clone(), v.clone())),
                None => ops.push(EnvOp::Unset(name.clone())),
            }
        }
    }

    // The value a variable would have *after* the restores above are applied.
    let effective = |name: &str| -> Option<String> {
        if let Some(state) = &state {
            if let Some(prev) = state.saved.get(name) {
                return prev.clone();
            }
        }
        current_env(name)
    };

    match target {
        Some(target) => {
            let mut saved = BTreeMap::new();
            let mut set = |ops: &mut Vec<EnvOp>, name: &str, value: &str| {
                saved.insert(name.to_string(), effective(name));
                ops.push(EnvOp::Set(name.to_string(), value.to_string()));
            };
            set(&mut ops, PROFILE_VAR, &target.profile);
            for (name, value) in &target.env {
                set(&mut ops, name, value);
            }
            let new_state = State {
                v: STATE_VERSION,
                profile: target.profile.clone(),
                saved,
            };
            ops.push(EnvOp::Set(STATE_VAR.to_string(), new_state.encode()));
        }
        None => {
            ops.push(EnvOp::Unset(STATE_VAR.to_string()));
        }
    }

    ops
}

/// Parse `GITID_STATE` from a raw env value, tolerating absence/corruption.
pub fn state_from_env(raw: Option<String>) -> Option<State> {
    raw.and_then(|s| State::decode(&s))
}

/// Validate that env values are representable across all shells (no newline/NUL).
pub fn check_env_value(name: &str, value: &str) -> Result<()> {
    if value.contains('\n') || value.contains('\0') {
        anyhow::bail!("env var {name} contains a newline or NUL and cannot be exported");
    }
    Ok(())
}

/// Convenience: a `current_env` closure backed by the real process environment.
pub fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(profile: &str, env: &[(&str, &str)]) -> Target {
        Target {
            profile: profile.into(),
            env: env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn env_none(_: &str) -> Option<String> {
        None
    }

    fn get(ops: &[EnvOp], name: &str) -> Option<EnvOp> {
        ops.iter()
            .find(|o| match o {
                EnvOp::Set(n, _) | EnvOp::Unset(n) => n == name,
            })
            .cloned()
    }

    #[test]
    fn none_to_none_is_noop() {
        assert!(compute_transition(None, None, &env_none).is_empty());
    }

    #[test]
    fn same_profile_is_noop() {
        let t = target("work", &[("GH_CONFIG_DIR", "/d")]);
        let state = State {
            v: 1,
            profile: "work".into(),
            saved: BTreeMap::new(),
        };
        assert!(compute_transition(Some(&t), Some(state), &env_none).is_empty());
    }

    #[test]
    fn activate_from_none_sets_vars_and_state() {
        let t = target("work", &[("GH_CONFIG_DIR", "/gh/work")]);
        let ops = compute_transition(Some(&t), None, &env_none);
        assert_eq!(
            get(&ops, "GITID_PROFILE"),
            Some(EnvOp::Set("GITID_PROFILE".into(), "work".into()))
        );
        assert_eq!(
            get(&ops, "GH_CONFIG_DIR"),
            Some(EnvOp::Set("GH_CONFIG_DIR".into(), "/gh/work".into()))
        );
        // State var is set and round-trips, recording prior (unset) values.
        let EnvOp::Set(_, encoded) = get(&ops, "GITID_STATE").unwrap() else {
            panic!()
        };
        let decoded = State::decode(&encoded).unwrap();
        assert_eq!(decoded.profile, "work");
        assert_eq!(decoded.saved["GITID_PROFILE"], None);
        assert_eq!(decoded.saved["GH_CONFIG_DIR"], None);
    }

    #[test]
    fn deactivate_restores_saved_values() {
        let mut saved = BTreeMap::new();
        saved.insert("GITID_PROFILE".to_string(), None);
        saved.insert(
            "GH_CONFIG_DIR".to_string(),
            Some("/user/global".to_string()),
        );
        let state = State {
            v: 1,
            profile: "work".into(),
            saved,
        };
        let ops = compute_transition(None, Some(state), &env_none);
        // GH_CONFIG_DIR restored to the user's prior value, profile unset.
        assert_eq!(
            get(&ops, "GH_CONFIG_DIR"),
            Some(EnvOp::Set("GH_CONFIG_DIR".into(), "/user/global".into()))
        );
        assert_eq!(
            get(&ops, "GITID_PROFILE"),
            Some(EnvOp::Unset("GITID_PROFILE".into()))
        );
        assert_eq!(
            get(&ops, "GITID_STATE"),
            Some(EnvOp::Unset("GITID_STATE".into()))
        );
    }

    #[test]
    fn switch_between_profiles_restores_then_activates() {
        // Active "work" saved a prior GH_CONFIG_DIR; switching to "personal"
        // which sets a different GH_CONFIG_DIR must record the restored value.
        let mut saved = BTreeMap::new();
        saved.insert("GITID_PROFILE".to_string(), None);
        saved.insert(
            "GH_CONFIG_DIR".to_string(),
            Some("/user/global".to_string()),
        );
        let state = State {
            v: 1,
            profile: "work".into(),
            saved,
        };
        let t = target("personal", &[("GH_CONFIG_DIR", "/gh/personal")]);
        let ops = compute_transition(Some(&t), Some(state), &env_none);
        assert_eq!(
            get(&ops, "GH_CONFIG_DIR"),
            Some(EnvOp::Set("GH_CONFIG_DIR".into(), "/gh/personal".into()))
        );
        let EnvOp::Set(_, encoded) = get(&ops, "GITID_STATE").unwrap() else {
            panic!()
        };
        let decoded = State::decode(&encoded).unwrap();
        // The new saved value is the *restored* user global, not work's dir.
        assert_eq!(
            decoded.saved["GH_CONFIG_DIR"],
            Some("/user/global".to_string())
        );
    }

    #[test]
    fn corrupt_state_decodes_to_none() {
        assert!(State::decode("!!!not base64!!!").is_none());
        assert!(State::decode("YWJj").is_none()); // "abc", not JSON
        // Wrong version.
        let bad = State {
            v: 99,
            profile: "x".into(),
            saved: BTreeMap::new(),
        };
        assert!(State::decode(&bad.encode()).is_none());
    }

    #[test]
    fn new_var_records_current_env_value() {
        // Activating where a var already has an inherited value (not from us).
        let current = |name: &str| -> Option<String> {
            if name == "GH_CONFIG_DIR" {
                Some("/preexisting".to_string())
            } else {
                None
            }
        };
        let t = target("work", &[("GH_CONFIG_DIR", "/gh/work")]);
        let ops = compute_transition(Some(&t), None, &current);
        let EnvOp::Set(_, encoded) = get(&ops, "GITID_STATE").unwrap() else {
            panic!()
        };
        let decoded = State::decode(&encoded).unwrap();
        assert_eq!(
            decoded.saved["GH_CONFIG_DIR"],
            Some("/preexisting".to_string())
        );
    }
}
