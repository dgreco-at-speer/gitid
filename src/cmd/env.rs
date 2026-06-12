//! `gitid env` — the shell-hook hot path.
//!
//! Reads the directory→profile mapping (one small file), diffs against the
//! `GITID_STATE` carried in the environment, and prints the activation in the
//! requested shell's syntax — or nothing when nothing changed. Must never break
//! a prompt: any error prints to stderr and exits 0 with no stdout.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;

use crate::activation::{STATE_VAR, Target, compute_transition, process_env, state_from_env};
use crate::cli::EnvArgs;
use crate::cmd::Ctx;
use crate::paths::PathStyle;
use crate::shell::render_ops;
use crate::store::mappings::{MappingsFile, match_dir};

pub fn run(ctx: &Ctx, args: &EnvArgs) -> Result<()> {
    // Escape hatch: a user can disable activation entirely.
    if std::env::var_os("GITID_DISABLE").is_some_and(|v| !v.is_empty()) {
        return Ok(());
    }

    // Resolve the directory, preferring the logical $PWD when it exists.
    let cwd = resolve_cwd(args.dir.as_deref());

    // Anything below that fails should not break the prompt.
    let output = match build_output(ctx, args, &cwd) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("gitid: {e:#}");
            return Ok(());
        }
    };
    print!("{output}");
    Ok(())
}

fn build_output(ctx: &Ctx, args: &EnvArgs, cwd: &std::path::Path) -> Result<String> {
    let mappings = MappingsFile::load(&ctx.paths.mappings_toml())?;
    let matched = match_dir(&mappings.mappings, cwd, PathStyle::host());

    let target = matched.map(|m| Target {
        profile: m.profile.clone(),
        env: filtered_env(&m.env),
    });

    let state = state_from_env(process_env(STATE_VAR));
    let ops = compute_transition(target.as_ref(), state, &process_env);
    Ok(render_ops(args.shell, &ops))
}

/// Drop any env entries that cannot be safely exported (newline/NUL); they are
/// rejected at sync time too, but be defensive on the hot path.
fn filtered_env(env: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    env.iter()
        .filter(|(_, v)| !v.contains('\n') && !v.contains('\0'))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Resolve the directory to match against: an explicit `--dir`, else the logical
/// `$PWD` if it points at an existing directory, else the real cwd.
fn resolve_cwd(dir: Option<&str>) -> PathBuf {
    if let Some(d) = dir {
        return PathBuf::from(d);
    }
    if let Some(pwd) = std::env::var_os("PWD") {
        let p = PathBuf::from(pwd);
        if p.is_dir() {
            return p;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
