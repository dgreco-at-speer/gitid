//! `gitid doctor` — diagnose configuration problems. Each check reports OK,
//! a warning, or a failure with a concrete fix hint. Exits non-zero if any
//! check fails.

use std::path::Path;
use std::process::{Command, ExitCode};

use anyhow::Result;
use owo_colors::OwoColorize;
use owo_colors::Stream::Stdout;

use crate::cli::DoctorArgs;
use crate::cmd::{Ctx, resolve_dir};
use crate::gitconfig::{
    MIN_GIT, config_get_with_origin, git_version, include_present, is_in_repo, render_fragment,
    render_include,
};
use crate::paths::PathStyle;
use crate::store::mappings::MappingsFile;
use crate::store::profiles::{self, SigningFormat};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Ok,
    Warn,
    Fail,
}

struct Report {
    failed: bool,
}

impl Report {
    fn new() -> Self {
        Self { failed: false }
    }

    fn check(&mut self, status: Status, msg: impl AsRef<str>, hint: Option<&str>) {
        let glyph = match status {
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
        println!("{glyph} {}", msg.as_ref());
        if let Some(h) = hint {
            println!(
                "    {} {h}",
                "→".if_supports_color(Stdout, |t| t.cyan().to_string())
            );
        }
        if status == Status::Fail {
            self.failed = true;
        }
    }
}

pub fn run(ctx: &Ctx, args: &DoctorArgs) -> Result<ExitCode> {
    let mut r = Report::new();

    check_git(&mut r);
    check_bootstrap(ctx, &mut r)?;
    check_artifacts(ctx, &mut r)?;
    check_mappings(ctx, &mut r, args)?;
    check_keys(ctx, &mut r)?;
    check_gh(ctx, &mut r);
    check_hook(ctx, &mut r);

    println!();
    if r.failed {
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

fn check_git(r: &mut Report) {
    match git_version() {
        Ok((maj, min)) if (maj, min) >= MIN_GIT => {
            r.check(Status::Ok, format!("git {maj}.{min}"), None);
        }
        Ok((maj, min)) => r.check(
            Status::Fail,
            format!("git {maj}.{min} is too old"),
            Some(&format!("upgrade git to >= {}.{}", MIN_GIT.0, MIN_GIT.1)),
        ),
        Err(e) => r.check(
            Status::Fail,
            format!("git not usable: {e:#}"),
            Some("install git"),
        ),
    }
}

fn check_bootstrap(ctx: &Ctx, r: &mut Report) -> Result<()> {
    let include = ctx.paths.include_gitconfig();
    match include_present(&ctx.paths.home, &include) {
        Ok(true) => r.check(Status::Ok, "global gitconfig includes gitid manifest", None),
        Ok(false) => r.check(
            Status::Fail,
            "global gitconfig does not include the gitid manifest",
            Some("run `gitid sync`"),
        ),
        Err(e) => r.check(
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
        let expected = render_fragment(profile, &ctx.paths.home);
        let path = ctx.paths.fragment(name);
        match std::fs::read_to_string(&path) {
            Ok(actual) if actual == expected => {}
            _ => stale = true,
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
        r.check(
            Status::Fail,
            "generated files are out of date",
            Some("run `gitid sync`"),
        );
    } else {
        r.check(Status::Ok, "generated files are up to date", None);
    }
    Ok(())
}

fn check_mappings(ctx: &Ctx, r: &mut Report, args: &DoctorArgs) -> Result<()> {
    let profiles = profiles::load(&ctx.paths.profiles_toml())?;
    let mappings = MappingsFile::load(&ctx.paths.mappings_toml())?;

    if mappings.mappings.is_empty() {
        r.check(
            Status::Warn,
            "no directory mappings configured",
            Some("`gitid use <profile> [dir]`"),
        );
    }

    for m in &mappings.mappings {
        let dir = m.dir.trim_end_matches('/');
        if !profiles.profiles.contains_key(&m.profile) {
            r.check(
                Status::Fail,
                format!("{dir} → unknown profile {:?}", m.profile),
                Some("fix profiles.toml or re-run `gitid use`"),
            );
            continue;
        }
        if !Path::new(dir).exists() {
            r.check(
                Status::Warn,
                format!("mapped directory missing: {dir}"),
                None,
            );
        }
    }

    // Precedence probe for the requested (or current) directory if it is a repo.
    let probe = resolve_dir(args.dir.as_deref(), &ctx.paths.home)?;
    if is_in_repo(&probe) {
        if let Some(mapping) =
            crate::store::mappings::match_dir(&mappings.mappings, &probe, PathStyle::host())
        {
            if let Some(profile) = profiles.profiles.get(&mapping.profile) {
                match config_get_with_origin(&probe, "user.email")? {
                    Some((actual, origin)) if actual != profile.email => r.check(
                        Status::Warn,
                        format!(
                            "git resolves user.email = {actual} here, not {}",
                            profile.email
                        ),
                        Some(&format!(
                            "overridden by {origin}; `git config --unset user.email` in that file"
                        )),
                    ),
                    Some((actual, _)) => r.check(
                        Status::Ok,
                        format!("git resolves {actual} in this repo"),
                        None,
                    ),
                    None => r.check(Status::Warn, "git resolves no user.email here", None),
                }
            }
        }
    }
    Ok(())
}

fn check_keys(ctx: &Ctx, r: &mut Report) -> Result<()> {
    let profiles = profiles::load(&ctx.paths.profiles_toml())?;
    for (name, profile) in &profiles.profiles {
        if let Some(ssh) = &profile.ssh {
            let path = crate::paths::expand_tilde(&ssh.key, &ctx.paths.home);
            if !path.exists() {
                r.check(
                    Status::Warn,
                    format!("{name}: ssh key not found: {}", path.display()),
                    None,
                );
            } else {
                check_perms(r, name, &path);
            }
        }
        if let Some(signing) = &profile.signing {
            if signing.format == SigningFormat::Ssh {
                let path = crate::paths::expand_tilde(&signing.key, &ctx.paths.home);
                if !path.exists() {
                    r.check(
                        Status::Warn,
                        format!("{name}: signing key not found: {}", path.display()),
                        None,
                    );
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
            r.check(
                Status::Warn,
                format!(
                    "{name}: ssh key {} is group/world accessible",
                    path.display()
                ),
                Some("chmod 600 the key"),
            );
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
    let gh_present = Command::new("gh")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !gh_present {
        r.check(
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
            r.check(Status::Ok, format!("{name}: gh auth configured"), None);
        } else {
            r.check(
                Status::Warn,
                format!("{name}: gh not authenticated"),
                Some(&format!(
                    "GH_CONFIG_DIR={} gh auth login",
                    ctx.paths.gh_dir(name).display()
                )),
            );
        }
    }
}

fn check_hook(ctx: &Ctx, r: &mut Report) {
    let candidates = [".bashrc", ".zshrc", ".config/fish/config.fish"];
    let installed = candidates.iter().any(|rc| {
        std::fs::read_to_string(ctx.paths.home.join(rc))
            .map(|c| c.contains("gitid hook"))
            .unwrap_or(false)
    });
    if installed {
        r.check(Status::Ok, "shell hook installed", None);
    } else {
        r.check(
            Status::Warn,
            "shell hook not detected in your rc files",
            Some("run `gitid setup` to install it"),
        );
    }
}
