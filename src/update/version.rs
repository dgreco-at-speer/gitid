//! Version parsing and comparison for the self-updater.
//!
//! Release tags are strictly `vX.Y.Z` (enforced by the `release.yml` tag regex)
//! and the in-binary version is `CARGO_PKG_VERSION` (`X.Y.Z`), so a plain
//! three-integer parse is sufficient — no `semver` dependency.

/// The version this binary was built as.
pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Parse a `vX.Y.Z` / `X.Y.Z` string into a comparable `(major, minor, patch)`
/// tuple. A leading `v` is tolerated. Returns `None` if the shape is unexpected.
fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.strip_prefix('v').unwrap_or(v);
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Whether `candidate` is strictly newer than `current`. If either side cannot be
/// parsed we conservatively return `false` (never nag or "update" toward a
/// version we don't understand).
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse(candidate), parse(current)) {
        (Some(c), Some(cur)) => c > cur,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_and_without_v() {
        assert_eq!(parse("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse("1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn rejects_malformed() {
        assert_eq!(parse("1.2"), None);
        assert_eq!(parse("1.2.3.4"), None);
        assert_eq!(parse("1.2.x"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn newer_detects_each_component() {
        assert!(is_newer("v0.3.0", "0.2.0"));
        assert!(is_newer("0.2.1", "0.2.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn not_newer_when_equal_or_older() {
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("v0.2.0", "0.2.0"));
        assert!(!is_newer("0.1.9", "0.2.0"));
    }

    #[test]
    fn malformed_is_never_newer() {
        assert!(!is_newer("garbage", "0.2.0"));
        assert!(!is_newer("0.3.0", "garbage"));
    }
}
