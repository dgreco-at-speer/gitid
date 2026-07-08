//! Which release asset matches the host this binary runs on.
//!
//! The running binary knows its own target at compile time, so we derive it from
//! `cfg!` rather than parsing `uname` (as the install scripts must). The triples
//! and archive extensions mirror `.github/workflows/release.yml` and
//! `scripts/install.sh` — including the "prefer musl, fall back to gnu" order on
//! Linux (musl is statically linked and runs on any libc).

/// A resolved release target: the preferred asset triple, an optional fallback
/// triple to try if the preferred asset is missing, and the archive extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// Preferred Rust triple, e.g. `x86_64-unknown-linux-musl`.
    pub triple: &'static str,
    /// Triple to try if `triple`'s asset is absent (e.g. gnu on Linux).
    pub fallback: Option<&'static str>,
    /// Archive extension without a leading dot, e.g. `tar.gz` or `zip`.
    pub ext: &'static str,
}

impl Target {
    /// The preferred triple followed by the fallback (if any), in order.
    pub fn triples(&self) -> impl Iterator<Item = &'static str> {
        std::iter::once(self.triple).chain(self.fallback)
    }
}

/// The release target for the host, or `None` when no prebuilt asset is produced
/// for this platform (e.g. arm64 Windows).
pub const fn current() -> Option<Target> {
    // Linux: prefer the static musl build, fall back to glibc (mirrors install.sh).
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Some(Target {
            triple: "x86_64-unknown-linux-musl",
            fallback: Some("x86_64-unknown-linux-gnu"),
            ext: "tar.gz",
        })
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        Some(Target {
            triple: "aarch64-unknown-linux-musl",
            fallback: Some("aarch64-unknown-linux-gnu"),
            ext: "tar.gz",
        })
    }
    // macOS (darwin assets are not published yet; the updater degrades gracefully
    // when the asset is missing — see cmd handling).
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        Some(Target {
            triple: "x86_64-apple-darwin",
            fallback: None,
            ext: "tar.gz",
        })
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Some(Target {
            triple: "aarch64-apple-darwin",
            fallback: None,
            ext: "tar.gz",
        })
    }
    // Windows: only x86_64 is built, packaged as a .zip.
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        Some(Target {
            triple: "x86_64-pc-windows-gnu",
            fallback: None,
            ext: "zip",
        })
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    {
        None
    }
}

/// Whether the host is macOS — used to tailor the "no prebuilt asset" message,
/// since darwin binaries are not published yet.
pub const fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_target_resolves_in_this_dev_env() {
        let t = current().expect("a target for the host");
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            assert_eq!(t.triple, "x86_64-unknown-linux-musl");
            assert_eq!(t.fallback, Some("x86_64-unknown-linux-gnu"));
            assert_eq!(t.ext, "tar.gz");
            let order: Vec<_> = t.triples().collect();
            assert_eq!(
                order,
                ["x86_64-unknown-linux-musl", "x86_64-unknown-linux-gnu"]
            );
        }
        let _ = t;
    }
}
