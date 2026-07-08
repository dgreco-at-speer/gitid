//! Command dispatch and shared context.

pub(crate) mod add;
mod completions;
mod current;
mod dirs;
mod doctor;
mod edit;
mod env;
mod hook;
mod list;
mod remove;
mod setup;
mod show;
pub(crate) mod sync;
mod use_dir;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;

use crate::cli::{Cli, Command};
use crate::paths::{GitidPaths, expand_tilde};

/// Shared state for command handlers.
pub struct Ctx {
    pub paths: GitidPaths,
}

impl Ctx {
    pub fn new() -> Result<Self> {
        Ok(Self {
            paths: GitidPaths::resolve()?,
        })
    }
}

/// Run the parsed CLI, returning a process exit code.
pub fn run(cli: Cli) -> Result<ExitCode> {
    let ctx = Ctx::new()?;
    let notify = should_notify(&cli.command);
    let code = dispatch(&ctx, cli.command)?;
    // Best-effort, notify-only update check after a user-facing command succeeds.
    if notify {
        crate::update::maybe_notify(&ctx);
    }
    Ok(code)
}

/// Dispatch a single command to its handler.
fn dispatch(ctx: &Ctx, command: Command) -> Result<ExitCode> {
    match command {
        Command::List(args) => list::run(ctx, &args)?,
        Command::Add(args) => add::run(ctx, &args)?,
        Command::Show(args) => show::run(ctx, &args)?,
        Command::Edit(args) => edit::run(ctx, &args)?,
        Command::Remove(args) => remove::run(ctx, &args)?,
        Command::Use(args) => use_dir::run(ctx, &args)?,
        Command::Forget(args) => use_dir::forget(ctx, &args)?,
        Command::Dirs(args) => dirs::run(ctx, &args)?,
        Command::Current(args) => return current::run(ctx, &args),
        Command::Doctor(args) => return doctor::run(ctx, &args),
        Command::Sync => sync::run(ctx)?,
        Command::Env(args) => env::run(ctx, &args)?,
        Command::Hook(args) => hook::run(&args)?,
        Command::Setup(args) => setup::run(ctx, &args)?,
        Command::Completions(args) => completions::run(&args)?,
        Command::Init => sync::init(ctx)?,
        Command::Update(args) => return crate::update::run(ctx, &args),
    }
    Ok(ExitCode::SUCCESS)
}

/// Whether to run the opportunistic update check after this command. Excludes
/// machine-facing / shell-eval'd commands (they must stay fast and emit no extra
/// bytes) and `update` itself (avoids recursion with the hidden refresh worker).
fn should_notify(command: &Command) -> bool {
    !matches!(
        command,
        Command::Env(_)
            | Command::Hook(_)
            | Command::Completions(_)
            | Command::Current(_)
            | Command::Update(_)
    )
}

/// Resolve a directory argument to an absolute path. A leading `~` is expanded;
/// relative paths are joined onto the current directory. The path is not
/// required to exist (use [`canonicalize_existing`] when it must).
pub fn resolve_dir(arg: Option<&str>, home: &Path) -> Result<PathBuf> {
    let raw = match arg {
        Some(a) => expand_tilde(a, home),
        None => std::env::current_dir()?,
    };
    if raw.is_absolute() {
        Ok(raw)
    } else {
        Ok(std::env::current_dir()?.join(raw))
    }
}
