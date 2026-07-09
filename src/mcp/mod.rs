//! The `gitid mcp` subtree: a Model Context Protocol server exposed over stdio
//! (`gitid mcp serve`, spawned per-session by an agent harness) plus a helper
//! that registers that server in each harness's config (`gitid mcp install`).
//!
//! The server never writes to stdout except through the rmcp JSON-RPC framing —
//! its tools call the domain layer (`store`, `sync`, `activation`) directly
//! rather than the `cmd::*` handlers, which print and may prompt.

mod install;
mod server;

use anyhow::{Context, Result};

use crate::cli::McpInstallArgs;
use crate::cmd::Ctx;

pub use install::render_config;

/// Run the MCP server over stdio until the harness closes the connection.
///
/// Async is confined to this function: we build a single-threaded Tokio runtime
/// and block on it, leaving the rest of the (synchronous) crate untouched.
pub fn serve(ctx: &Ctx) -> Result<()> {
    let paths = ctx.paths.clone();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("could not start the async runtime for the MCP server")?;
    rt.block_on(server::run(paths))
}

/// Register `gitid mcp serve` as an MCP server in the requested harness configs.
pub fn install(ctx: &Ctx, args: &McpInstallArgs) -> Result<()> {
    install::run(ctx, args)
}
