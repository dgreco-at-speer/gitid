//! `gitid doctor` — diagnose configuration problems. Each check reports OK,
//! a warning, or a failure with a concrete fix hint. Exits non-zero if any
//! check fails. Output is split into a `Global` section, a per-profile health
//! table (SSH / signing / GH), and a `details` list of per-profile findings.

use std::path::Path;
use std::process::{Command, ExitCode};

use anyhow::Result;
use owo_colors::OwoColorize;
use owo_colors::Stream::Stdout;
use serde::Serialize;

use crate::cli::DoctorArgs;
use crate::cmd::{Ctx, resolve_dir};
use crate::gitconfig::{
    BootstrapStatus, MIN_GIT, bootstrap_status, config_get_with_origin, git_version, is_in_repo,
    render_fragment, render_include,
};
use crate::paths::PathStyle;
use crate::store::mappings::MappingsFile;
use crate::store::profiles::{self, SIGNING_KEY_AGENT, SigningFormat};

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

/// A single diagnostic result. Collected by [`collect`] so both the CLI printer
/// and the `gitid_doctor` MCP tool can consume the same checks.
#[derive(Clone, Serialize)]
pub struct Finding {
    pub status: Status,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// A table-cell status, coarser than [`Status`]: `Na` means the feature isn't
/// configured for the profile at all, so there is nothing to report.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cell {
    Ok,
    Warn,
    Fail,
    Na,
}

impl Cell {
    fn glyph(self) -> &'static str {
        match self {
            Cell::Ok => "✓",
            Cell::Warn => "!",
            Cell::Fail => "✗",
            Cell::Na => "—",
        }
    }
}

/// One row of the per-profile health table.
#[derive(Clone)]
pub struct ProfileRow {
    pub name: String,
    pub ssh: Cell,
    pub signing: Cell,
    pub gh: Cell,
}

/// Diagnostic results, sectioned for display: instance-wide findings, a
/// per-profile health table, and per-profile findings that need attention.
#[derive(Default)]
pub struct Report {
    /// git, bootstrap, generated-artifact freshness, mappings, and shell
    /// hook/completions detection — none of these are per-profile.
    pub global: Vec<Finding>,
    /// One row per configured profile, always present (even when healthy).
    pub profiles: Vec<ProfileRow>,
    /// Per-profile findings; messages are already prefixed `"{name}: "`.
    pub details: Vec<Finding>,
}

impl Report {
    fn global(&mut self, status: Status, msg: impl AsRef<str>, hint: Option<&str>) {
        self.global.push(Finding {
            status,
            message: msg.as_ref().to_string(),
            hint: hint.map(str::to_string),
        });
    }

    fn detail(&mut self, status: Status, msg: impl AsRef<str>, hint: Option<&str>) {
        self.details.push(Finding {
            status,
            message: msg.as_ref().to_string(),
            hint: hint.map(str::to_string),
        });
    }

    fn row_mut(&mut self, name: &str) -> Option<&mut ProfileRow> {
        self.profiles.iter_mut().find(|p| p.name == name)
    }

    /// Whether any check failed (drives the CLI's non-zero exit).
    pub fn failed(&self) -> bool {
        self.global
            .iter()
            .chain(self.details.iter())
            .any(|f| f.status == Status::Fail)
    }

    /// Flat findings list — the `gitid_doctor` MCP tool's JSON contract
    /// predates the sectioned report and keeps this shape.
    pub fn all_findings(&self) -> Vec<Finding> {
        self.global
            .iter()
            .chain(self.details.iter())
            .cloned()
            .collect()
    }
}

/// Run every diagnostic check and return the findings without emitting output.
pub fn collect(ctx: &Ctx, dir: Option<&str>) -> Result<Report> {
    let mut r = Report::default();

    check_git(&mut r);
    check_bootstrap(ctx, &mut r)?;
    check_artifacts(ctx, &mut r)?;
    check_mappings(ctx, &mut r, dir)?;

    // One row per profile, initialized to "not configured" so every profile
    // shows in the health table even when it has nothing to warn about.
    let profiles = profiles::load(&ctx.paths.profiles_toml())?;
    r.profiles = profiles
        .profiles
        .keys()
        .map(|name| ProfileRow {
            name: name.clone(),
            ssh: Cell::Na,
            signing: Cell::Na,
            gh: Cell::Na,
        })
        .collect();

    check_keys(ctx, &mut r)?;
    check_agent(ctx, &mut r)?;
    check_gh(ctx, &mut r);

    check_shell_feature(
        &mut r.global,
        "shell hook",
        "GITID_HOOK_ACTIVE",
        "gitid hook",
        "run `gitid setup`",
        &ctx.paths.home,
    );
    check_shell_feature(
        &mut r.global,
        "shell completions",
        "GITID_COMPLETIONS_ACTIVE",
        "gitid completions",
        "run `gitid completions <shell>` and source it (see `gitid completions <shell> --help`)",
        &ctx.paths.home,
    );

    Ok(r)
}

pub fn run(ctx: &Ctx, args: &DoctorArgs) -> Result<ExitCode> {
    let report = collect(ctx, args.dir.as_deref())?;

    println!(
        "{}",
        "Global".if_supports_color(Stdout, |t| t.bold().to_string())
    );
    for f in &report.global {
        print_finding(f);
    }

    if !report.profiles.is_empty() {
        let active = active_profile_row(ctx, args.dir.as_deref(), &report.profiles);
        let rows: Vec<Vec<String>> = report
            .profiles
            .iter()
            .map(|p| {
                vec![
                    p.name.clone(),
                    p.ssh.glyph().to_string(),
                    p.signing.glyph().to_string(),
                    p.gh.glyph().to_string(),
                ]
            })
            .collect();
        println!();
        print!(
            "{}",
            crate::output::table_with_active(&["PROFILE", "SSH", "SIGNING", "GH"], &rows, active)
        );
    }

    if !report.details.is_empty() {
        println!();
        for f in &report.details {
            print_finding(f);
        }
    }

    println!();
    if report.failed() {
        println!(
            "{}",
            "doctor found problems (see ✗ above)"
                .if_supports_color(Stdout, |t| t.red().to_string())
        );
        Ok(ExitCode::FAILURE)
    } else {
        println!(
            "{}",
            "all checks passed".if_supports_color(Stdout, |t| t.green().to_string())
        );
        Ok(ExitCode::SUCCESS)
    }
}

fn print_finding(f: &Finding) {
    let glyph = match f.status {
        Status::Ok => "✓"
            .if_supports_color(Stdout, |t| t.green().to_string())
            .to_string(),
        Status::Warn => "!"
            .if_supports_color(Stdout, |t| t.yellow().to_string())
            .to_string(),
        Status::Fail => "✗"
            .if_supports_color(Stdout, |t| t.red().to_string())
            .to_string(),
    };
    println!("{glyph} {}", f.message);
    if let Some(h) = &f.hint {
        println!(
            "    {} {h}",
            "→".if_supports_color(Stdout, |t| t.cyan().to_string())
        );
    }
}

/// Which profile (if any) governs `dir` — used to highlight its row in the
/// health table, using the same mapping precedence `check_mappings` probes
/// for the `user.email` check.
fn active_profile_row(ctx: &Ctx, dir: Option<&str>, rows: &[ProfileRow]) -> Option<usize> {
    let probe = resolve_dir(dir, &ctx.paths.home).ok()?;
    let mappings = MappingsFile::load(&ctx.paths.mappings_toml()).ok()?;
    let mapping = crate::store::mappings::match_dir(&mappings.mappings, &probe, PathStyle::host())?;
    rows.iter().position(|row| row.name == mapping.profile)
}

fn check_git(r: &mut Report) {
    match git_version() {
        Ok((maj, min)) if (maj, min) >= MIN_GIT => {
            r.global(Status::Ok, format!("git {maj}.{min}"), None);
        }
        Ok((maj, min)) => r.global(
            Status::Fail,
            format!("git {maj}.{min} is too old"),
            Some(&format!("upgrade git to >= {}.{}", MIN_GIT.0, MIN_GIT.1)),
        ),
        Err(e) => r.global(
            Status::Fail,
            format!("git not usable: {e:#}"),
            Some("install git"),
        ),
    }
}

fn check_bootstrap(ctx: &Ctx, r: &mut Report) -> Result<()> {
    let include = ctx.paths.include_gitconfig();
    match bootstrap_status(&ctx.paths.home, &include) {
        Ok(BootstrapStatus::Present) => {
            r.global(Status::Ok, "global gitconfig includes gitid manifest", None)
        }
        Ok(BootstrapStatus::PresentViaLocal { local }) => r.global(
            Status::Ok,
            format!("gitid manifest included via {}", local.display()),
            None,
        ),
        Ok(BootstrapStatus::LocalNotLoaded { global, local }) => r.global(
            Status::Warn,
            format!(
                "{} is read-only; gitid wrote its include to {} but git does not load it",
                global.display(),
                local.display()
            ),
            Some("include that file from your managed global git config"),
        ),
        Ok(BootstrapStatus::Missing) => r.global(
            Status::Fail,
            "global gitconfig does not include the gitid manifest",
            Some("run `gitid sync`"),
        ),
        Err(e) => r.global(
            Status::Warn,
            format!("could not check global include: {e:#}"),
            None,
        ),
    }
    Ok(())
}

fn check_artifacts(ctx: &Ctx, r: &mut Report) -> Result<()> {
    let profiles = profiles::load(&ctx.paths.profiles_toml())?;
    let mappings = MappingsFile::load(&ctx.paths.mappings_toml())?;

    // Fragments match what sync would generate.
    let mut stale = false;
    for (name, profile) in &profiles.profiles {
        let expected = render_fragment(name, profile, &ctx.paths);
        let path = ctx.paths.fragment(name);
        match std::fs::read_to_string(&path) {
            Ok(actual) if actual == expected => {}
            _ => stale = true,
        }
        // Agent-held keys must have been materialised (freshness vs the live
        // agent is checked separately, without failing offline).
        if profile
            .ssh
            .as_ref()
            .is_some_and(|s| s.agent_selector().is_some())
            && !ctx.paths.ssh_pub(name).exists()
        {
            stale = true;
        }
    }
    let expected_include = render_include(
        &mappings.mappings,
        &ctx.paths.home.to_string_lossy(),
        PathStyle::host(),
    );
    if std::fs::read_to_string(ctx.paths.include_gitconfig())
        .ok()
        .as_deref()
        != Some(&expected_include)
    {
        stale = true;
    }
    if stale {
        r.global(
            Status::Fail,
            "generated files are out of date",
            Some("run `gitid sync`"),
        );
    } else {
        r.global(Status::Ok, "generated files are up to date", None);
    }
    Ok(())
}

fn check_mappings(ctx: &Ctx, r: &mut Report, dir: Option<&str>) -> Result<()> {
    let profiles = profiles::load(&ctx.paths.profiles_toml())?;
    let mappings = MappingsFile::load(&ctx.paths.mappings_toml())?;

    if mappings.mappings.is_empty() {
        r.global(
            Status::Warn,
            "no directory mappings configured",
            Some("`gitid use <profile> [dir]`"),
        );
    }

    for m in &mappings.mappings {
        let dir = m.dir.trim_end_matches('/');
        if !profiles.profiles.contains_key(&m.profile) {
            r.global(
                Status::Fail,
                format!("{dir} → unknown profile {:?}", m.profile),
                Some("fix profiles.toml or re-run `gitid use`"),
            );
            continue;
        }
        if !Path::new(dir).exists() {
            r.global(
                Status::Warn,
                format!("mapped directory missing: {dir}"),
                None,
            );
        }
    }

    // Precedence probe for the requested (or current) directory if it is a repo.
    let probe = resolve_dir(dir, &ctx.paths.home)?;
    if is_in_repo(&probe) {
        if let Some(mapping) =
            crate::store::mappings::match_dir(&mappings.mappings, &probe, PathStyle::host())
        {
            if let Some(profile) = profiles.profiles.get(&mapping.profile) {
                match config_get_with_origin(&probe, "user.email")? {
                    Some((actual, origin)) if actual != profile.email => r.global(
                        Status::Warn,
                        format!(
                            "git resolves user.email = {actual} here, not {}",
                            profile.email
                        ),
                        Some(&format!(
                            "overridden by {origin}; `git config --unset user.email` in that file"
                        )),
                    ),
                    Some((actual, _)) => r.global(
                        Status::Ok,
                        format!("git resolves {actual} in this repo"),
                        None,
                    ),
                    None => r.global(Status::Warn, "git resolves no user.email here", None),
                }
            }
        }
    }
    Ok(())
}

fn check_keys(ctx: &Ctx, r: &mut Report) -> Result<()> {
    let profiles = profiles::load(&ctx.paths.profiles_toml())?;

    // SSH signing needs git >= 2.34 (user.signingkey + gpg.format = ssh).
    let any_ssh_signing = profiles.profiles.values().any(|p| {
        p.signing
            .as_ref()
            .is_some_and(|s| s.format == SigningFormat::Ssh)
    });
    if any_ssh_signing {
        if let Ok((maj, min)) = git_version() {
            if (maj, min) < (2, 34) {
                r.global(
                    Status::Warn,
                    format!("git {maj}.{min} does not support SSH commit signing"),
                    Some("upgrade git to >= 2.34"),
                );
            }
        }
    }

    for (name, profile) in &profiles.profiles {
        // Agent-held keys have no file to stat; check_agent covers them.
        if let Some(key) = profile.ssh.as_ref().and_then(|s| s.path()) {
            let path = crate::paths::expand_tilde(key, &ctx.paths.home);
            if !path.exists() {
                r.detail(
                    Status::Warn,
                    format!("{name}: ssh key not found: {}", path.display()),
                    None,
                );
                if let Some(row) = r.row_mut(name) {
                    row.ssh = Cell::Warn;
                }
            } else {
                if let Some(row) = r.row_mut(name) {
                    row.ssh = Cell::Ok;
                }
                check_perms(r, name, &path);
            }
        }
        if let Some(signing) = &profile.signing {
            if let Some(row) = r.row_mut(name) {
                row.signing = Cell::Ok;
            }
            if signing.format == SigningFormat::Ssh {
                if signing.key == SIGNING_KEY_AGENT {
                    if profile
                        .ssh
                        .as_ref()
                        .and_then(|s| s.agent_selector())
                        .is_none()
                    {
                        r.detail(
                            Status::Warn,
                            format!(
                                "{name}: signing key is \"agent\" but the profile's ssh key \
                                 is not agent-held"
                            ),
                            Some(
                                "use `agent = \"…\"` in the profile's ssh table, or point \
                                 signing.key at a public key file",
                            ),
                        );
                        if let Some(row) = r.row_mut(name) {
                            row.signing = Cell::Warn;
                        }
                    }
                } else {
                    let path = crate::paths::expand_tilde(&signing.key, &ctx.paths.home);
                    if !path.exists() {
                        r.detail(
                            Status::Warn,
                            format!("{name}: signing key not found: {}", path.display()),
                            None,
                        );
                        if let Some(row) = r.row_mut(name) {
                            row.signing = Cell::Warn;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Check that every agent-held key is actually present in the running agent and
/// that its materialised public key is fresh. Unreachable agents are warnings —
/// the derived config keeps working from the cached key.
fn check_agent(ctx: &Ctx, r: &mut Report) -> Result<()> {
    let profiles = profiles::load(&ctx.paths.profiles_toml())?;
    let agent_profiles: Vec<(&String, &str)> = profiles
        .profiles
        .iter()
        .filter_map(|(name, p)| {
            p.ssh
                .as_ref()
                .and_then(|s| s.agent_selector())
                .map(|sel| (name, sel))
        })
        .collect();
    if agent_profiles.is_empty() {
        return Ok(());
    }

    let keys = crate::output::with_spinner("checking ssh-agent…", crate::agent::list_keys);
    let keys = match keys {
        Ok(keys) => keys,
        Err(e) => {
            let hint = if cfg!(windows) {
                "start the \"OpenSSH Authentication Agent\" service; note that Git for \
                 Windows' bundled ssh cannot talk to it — put Windows OpenSSH first on PATH"
            } else {
                "start ssh-agent and set SSH_AUTH_SOCK"
            };
            r.global(
                Status::Warn,
                format!("ssh-agent not reachable: {e:#}"),
                Some(hint),
            );
            for (name, _) in &agent_profiles {
                if !ctx.paths.ssh_pub(name).exists() {
                    r.global(
                        Status::Fail,
                        format!("{name}: agent key was never materialised"),
                        Some("run `gitid sync` while the agent is running"),
                    );
                    if let Some(row) = r.row_mut(name) {
                        row.ssh = Cell::Fail;
                    }
                } else if let Some(row) = r.row_mut(name) {
                    row.ssh = Cell::Warn;
                }
            }
            return Ok(());
        }
    };

    for (name, selector) in agent_profiles {
        match crate::agent::select(&keys, selector) {
            Ok(key) => {
                let expected = format!("{}\n", key.line());
                match std::fs::read_to_string(ctx.paths.ssh_pub(name)) {
                    Ok(actual) if actual == expected => {
                        r.detail(
                            Status::Ok,
                            format!("{name}: ssh-agent holds {}", key.fingerprint()),
                            None,
                        );
                        if let Some(row) = r.row_mut(name) {
                            row.ssh = Cell::Ok;
                        }
                    }
                    _ => {
                        r.detail(
                            Status::Warn,
                            format!("{name}: materialised agent key is stale or missing"),
                            Some("run `gitid sync`"),
                        );
                        if let Some(row) = r.row_mut(name) {
                            row.ssh = Cell::Warn;
                        }
                    }
                }
            }
            Err(e) => {
                r.detail(
                    Status::Warn,
                    format!("{name}: {e:#}"),
                    Some("check `ssh-add -l`, then run `gitid sync`"),
                );
                if let Some(row) = r.row_mut(name) {
                    row.ssh = Cell::Warn;
                }
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn check_perms(r: &mut Report, name: &str, path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mode = meta.permissions().mode() & 0o077;
        if mode != 0 {
            r.detail(
                Status::Warn,
                format!(
                    "{name}: ssh key {} is group/world accessible",
                    path.display()
                ),
                Some("chmod 600 the key"),
            );
            if let Some(row) = r.row_mut(name) {
                row.ssh = Cell::Warn;
            }
        }
    }
}

#[cfg(not(unix))]
fn check_perms(_r: &mut Report, _name: &str, _path: &Path) {}

fn check_gh(ctx: &Ctx, r: &mut Report) {
    let Ok(profiles) = profiles::load(&ctx.paths.profiles_toml()) else {
        return;
    };
    let any_gh = profiles.profiles.values().any(|p| p.gh_enabled());
    if !any_gh {
        return;
    }
    let gh_present = crate::output::with_spinner("checking gh…", || {
        Command::new("gh")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    });
    if !gh_present {
        r.global(
            Status::Warn,
            "gh CLI not found but profiles enable gh isolation",
            Some("install GitHub CLI, or set gh.enabled = false"),
        );
        return;
    }
    for (name, profile) in &profiles.profiles {
        if !profile.gh_enabled() {
            continue;
        }
        let hosts = ctx.paths.gh_dir(name).join("hosts.yml");
        if hosts.exists() {
            r.detail(Status::Ok, format!("{name}: gh auth configured"), None);
            if let Some(row) = r.row_mut(name) {
                row.gh = Cell::Ok;
            }
        } else {
            r.detail(
                Status::Warn,
                format!("{name}: gh not authenticated"),
                Some(&format!(
                    "GH_CONFIG_DIR={} gh auth login",
                    ctx.paths.gh_dir(name).display()
                )),
            );
            if let Some(row) = r.row_mut(name) {
                row.gh = Cell::Warn;
            }
        }
    }
}

/// Three-state install detection shared by the shell hook and shell
/// completions checks: active (env var set by the sourced script), configured
/// but not active in this shell (rc file references the install line but the
/// var isn't set — e.g. a fresh non-interactive shell), or not installed at
/// all. Both absent states are warnings, never failures.
fn check_shell_feature(
    global: &mut Vec<Finding>,
    label: &str,
    env_var: &str,
    rc_marker: &str,
    install_hint: &str,
    home: &Path,
) {
    match std::env::var(env_var) {
        Ok(v) if !v.is_empty() => global.push(Finding {
            status: Status::Ok,
            message: format!("{label} active"),
            hint: None,
        }),
        _ => {
            let rcs = [
                ".bashrc",
                ".zshrc",
                ".config/fish/config.fish",
                ".config/powershell/Microsoft.PowerShell_profile.ps1",
                ".config/nushell/vendor/autoload/gitid.nu",
            ];
            let configured = rcs.iter().any(|rc| {
                std::fs::read_to_string(home.join(rc))
                    .map(|c| c.contains(rc_marker))
                    .unwrap_or(false)
            });
            if configured {
                global.push(Finding {
                    status: Status::Warn,
                    message: format!("{label} configured but not active in this shell"),
                    hint: Some("start a new shell or re-source your rc file".into()),
                });
            } else {
                global.push(Finding {
                    status: Status::Warn,
                    message: format!("{label} not installed"),
                    hint: Some(install_hint.into()),
                });
            }
        }
    }
}
