//! The update-check throttle cache.
//!
//! Machine-owned, lives under the data dir, and records when we last checked and
//! the newest version we saw. Written as JSON via the existing `serde_json`
//! dependency and the crate's atomic-write primitive. Follows the versioned
//! schema pattern used by [`crate::store::mappings`].

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::store::atomic_write;

pub const VERSION: u32 = 1;

/// Cached result of the last update check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateState {
    pub version: u32,
    /// Unix seconds of the last check attempt (success or failure). `0` = never.
    #[serde(default)]
    pub last_check: u64,
    /// The newest release tag observed on the last successful check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            version: VERSION,
            last_check: 0,
            latest_version: None,
        }
    }
}

impl UpdateState {
    /// Load the cache, treating a missing/unreadable/incompatible file as an
    /// empty default — this is a best-effort convenience cache, never a hard
    /// error surface.
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str::<UpdateState>(&text) {
            Ok(s) if s.version == VERSION => s,
            _ => Self::default(),
        }
    }

    /// Persist atomically. Callers that fail to save just lose the throttle for
    /// one cycle, so this is only surfaced where useful.
    pub fn save(&self, path: &Path) -> Result<()> {
        let text =
            serde_json::to_string_pretty(self).context("could not serialise update state")?;
        atomic_write(path, &text)
    }

    /// Whether a fresh check is due given `interval` seconds since `last_check`.
    pub fn is_stale(&self, interval: u64, now: u64) -> bool {
        now.saturating_sub(self.last_check) >= interval
    }
}

/// Current wall-clock time in unix seconds (saturating; monotonic enough for a
/// day-granularity throttle).
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_default() {
        let s = UpdateState::load(Path::new("/no/such/gitid-update.json"));
        assert_eq!(s.last_check, 0);
        assert!(s.latest_version.is_none());
    }

    #[test]
    fn round_trips_through_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("update-check.json");
        let s = UpdateState {
            version: VERSION,
            last_check: 1_000,
            latest_version: Some("v0.3.0".into()),
        };
        s.save(&path).unwrap();
        let back = UpdateState::load(&path);
        assert_eq!(back.last_check, 1_000);
        assert_eq!(back.latest_version.as_deref(), Some("v0.3.0"));
    }

    #[test]
    fn incompatible_version_is_default() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("update-check.json");
        std::fs::write(&path, r#"{"version":999,"last_check":5}"#).unwrap();
        assert_eq!(UpdateState::load(&path).last_check, 0);
    }

    #[test]
    fn staleness() {
        let s = UpdateState {
            version: VERSION,
            last_check: 100,
            latest_version: None,
        };
        assert!(!s.is_stale(50, 120)); // 20s elapsed < 50s interval
        assert!(s.is_stale(50, 160)); // 60s elapsed >= 50s interval
        // Never-checked (last_check 0) is stale against any real wall-clock now.
        assert!(UpdateState::default().is_stale(50, 1_000_000));
    }
}
