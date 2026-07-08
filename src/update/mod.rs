//! Self-update: the `update` subcommand plus the opportunistic, notify-only
//! background check wired into normal command dispatch.
//!
//! The foreground never blocks on the network: the opportunistic check only ever
//! *reads* a cached result (written by a previous run) and, when that cache is
//! stale, spawns a detached child (`gitid update --refresh-cache`) to refresh it
//! for next time. Explicit `gitid update` / `update --check` do hit the network,
//! wrapped in a spinner.

mod github;
mod state;
mod target;
mod version;

use std::io::IsTerminal;
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result};

use crate::cli::UpdateArgs;
use crate::cmd::Ctx;
use crate::output;

/// Default throttle for the opportunistic check: at most once per 24h.
const DEFAULT_INTERVAL_SECS: u64 = 24 * 60 * 60;

/// Entry point for `gitid update`.
pub fn run(ctx: &Ctx, args: &UpdateArgs) -> Result<ExitCode> {
    if args.refresh_cache {
        // Hidden worker spawned by `maybe_notify`: refresh the cache silently.
        refresh_cache(ctx);
        return Ok(ExitCode::SUCCESS);
    }
    if args.check {
        return check(ctx);
    }
    install(ctx, args)
}

/// `gitid update --check`: report status, refresh the cache, install nothing.
fn check(ctx: &Ctx) -> Result<ExitCode> {
    let repo = github::repo();
    let latest = output::with_spinner("checking for updates…", || github::latest_tag(&repo))?;
    record(ctx, Some(&latest));

    let current = version::current();
    if version::is_newer(&latest, current) {
        output::info(&format!(
            "gitid {} is available (you have {current}). Run `gitid update`.",
            latest.trim_start_matches('v'),
        ));
    } else {
        output::success(&format!("gitid is up to date ({current})."));
    }
    Ok(ExitCode::SUCCESS)
}

/// `gitid update`: download the target release and replace the running binary.
fn install(ctx: &Ctx, args: &UpdateArgs) -> Result<ExitCode> {
    let repo = github::repo();
    let current = version::current();

    // Which version: explicit --version, else $GITID_VERSION, else latest.
    let pinned = args.version.clone().or_else(|| {
        std::env::var("GITID_VERSION")
            .ok()
            .filter(|v| !v.is_empty())
    });
    let wants_specific = pinned.is_some();
    let tag = match pinned {
        Some(v) => v,
        None => output::with_spinner("checking for updates…", || github::latest_tag(&repo))?,
    };
    record(ctx, Some(&tag));

    if !args.force {
        if wants_specific {
            if tag.trim_start_matches('v') == current {
                output::success(&format!("gitid is already at {current}."));
                return Ok(ExitCode::SUCCESS);
            }
        } else if !version::is_newer(&tag, current) {
            output::success(&format!("gitid is up to date ({current})."));
            return Ok(ExitCode::SUCCESS);
        }
    }

    let target = target::current().ok_or_else(|| {
        anyhow::anyhow!(
            "no prebuilt binary is published for this platform ({}/{}); reinstall from source: \
             cargo install --git https://github.com/{repo} gitid",
            std::env::consts::OS,
            std::env::consts::ARCH,
        )
    })?;

    let tmp = tempfile::tempdir().context("could not create temp dir")?;
    let bin = output::with_spinner(&format!("downloading gitid {tag}…"), || -> Result<_> {
        let archive = github::download_asset(&repo, &tag, &target, tmp.path())?;
        github::extract_binary(&archive, tmp.path())
    })
    .map_err(|e| macos_hint(e, &repo))?;

    self_replace::self_replace(&bin).context("could not replace the running gitid binary")?;

    output::success(&format!("updated gitid to {}", tag.trim_start_matches('v')));
    Ok(ExitCode::SUCCESS)
}

/// On macOS (no prebuilt asset yet), add a build-from-source hint to a download
/// failure so the message is actionable.
fn macos_hint(err: anyhow::Error, repo: &str) -> anyhow::Error {
    if target::is_macos() {
        err.context(format!(
            "no prebuilt macOS binary yet — reinstall from source: \
             cargo install --git https://github.com/{repo} gitid"
        ))
    } else {
        err
    }
}

/// The silent worker: do the network check and update the cache. Always bumps
/// `last_check` (even on failure) so we don't hammer when offline.
fn refresh_cache(ctx: &Ctx) {
    let latest = github::latest_tag(&github::repo()).ok();
    record(ctx, latest.as_deref());
}

/// Best-effort opportunistic check, hooked into normal command dispatch. Prints a
/// notice from the *cached* result (never blocks) and, if the cache is stale,
/// spawns a detached refresh for next time.
pub fn maybe_notify(ctx: &Ctx) {
    if disabled() {
        return;
    }
    let path = ctx.paths.update_state_json();
    let st = state::UpdateState::load(&path);

    // Notice from the previous run's cached result. On stderr so it never
    // corrupts stdout consumed by pipelines.
    if let Some(latest) = &st.latest_version {
        if version::is_newer(latest, version::current()) {
            output::note(&format!(
                "gitid {} is available (you have {}). Run `gitid update`.",
                latest.trim_start_matches('v'),
                version::current(),
            ));
        }
    }

    // Only probe the network from interactive sessions — never spawn background
    // work from scripts or CI.
    if std::io::stderr().is_terminal() && st.is_stale(interval(), state::now_unix()) {
        spawn_detached_refresh();
    }
}

/// Persist the check result: stamp `last_check` now and record `latest` if known.
fn record(ctx: &Ctx, latest: Option<&str>) {
    let path = ctx.paths.update_state_json();
    let mut st = state::UpdateState::load(&path);
    st.version = state::VERSION;
    st.last_check = state::now_unix();
    if let Some(l) = latest {
        st.latest_version = Some(l.to_string());
    }
    let _ = st.save(&path);
}

/// Whether the opportunistic check is disabled via `$GITID_NO_UPDATE_CHECK`.
fn disabled() -> bool {
    std::env::var_os("GITID_NO_UPDATE_CHECK").is_some_and(|v| !v.is_empty())
}

/// Throttle interval in seconds, overridable via `$GITID_UPDATE_INTERVAL`.
fn interval() -> u64 {
    std::env::var("GITID_UPDATE_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_INTERVAL_SECS)
}

/// Spawn `gitid update --refresh-cache` fully detached, with all stdio nulled, and
/// do not wait on it — the network check happens out of the foreground's way.
fn spawn_detached_refresh() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut cmd = Command::new(exe);
    cmd.args(["update", "--refresh-cache"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut cmd);
    let _ = cmd.spawn();
}

#[cfg(windows)]
fn detach(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn detach(_cmd: &mut Command) {}
