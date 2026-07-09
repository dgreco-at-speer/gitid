//! `gitid mcp install` — register `gitid mcp serve` as an MCP server in agent
//! harness config files. Generalises the consent/idempotency UX of
//! `gitid setup` across harnesses, each of which stores servers under its own
//! key with its own entry shape.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use crate::cli::{McpClient, McpInstallArgs};
use crate::cmd::Ctx;
use crate::output;
use crate::store::atomic_write;

/// The name gitid registers itself under in every harness's server map.
const SERVER_KEY: &str = "gitid";

pub fn run(ctx: &Ctx, args: &McpInstallArgs) -> Result<()> {
    let exe = current_exe_string()?;
    let clients: Vec<McpClient> = if args.client.is_empty() {
        McpClient::ALL.to_vec()
    } else {
        args.client.clone()
    };
    for client in clients {
        configure(ctx, client, &exe, args)?;
    }
    Ok(())
}

fn configure(ctx: &Ctx, client: McpClient, exe: &str, args: &McpInstallArgs) -> Result<()> {
    let path = config_path(ctx, client, args.project);

    if args.print {
        output::info(&format!("{} → {}", label(client), path.display()));
        let snippet = json!({ servers_key(client): { SERVER_KEY: entry(client, exe) } });
        println!("{}", serde_json::to_string_pretty(&snippet)?);
        return Ok(());
    }

    let existing = read_json(&path)?;
    let updated = render_config(existing.clone(), client, exe);
    if existing == updated {
        output::success(&format!(
            "{}: gitid already registered in {}",
            label(client),
            path.display()
        ));
        return Ok(());
    }

    output::info(&format!(
        "will register gitid ({}) in {}",
        label(client),
        path.display()
    ));
    if !args.yes && !confirm(&format!("Write {}?", path.display()))? {
        output::info("skipped");
        return Ok(());
    }

    let text = format!("{}\n", serde_json::to_string_pretty(&updated)?);
    atomic_write(&path, &text)?;
    output::success(&format!(
        "{}: registered gitid in {}",
        label(client),
        path.display()
    ));
    Ok(())
}

/// Merge the gitid server entry into an existing (or empty) harness config,
/// preserving every other key. Pure and unit-tested.
pub fn render_config(mut existing: Value, client: McpClient, exe: &str) -> Value {
    if !existing.is_object() {
        existing = Value::Object(Map::new());
    }
    let obj = existing.as_object_mut().expect("object");
    let was_empty = obj.is_empty();

    let servers = obj
        .entry(servers_key(client).to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !servers.is_object() {
        *servers = Value::Object(Map::new());
    }
    servers
        .as_object_mut()
        .expect("object")
        .insert(SERVER_KEY.to_string(), entry(client, exe));

    // Seed OpenCode's schema pointer only when we are creating the file fresh,
    // so we never rewrite an existing user config beyond the server entry.
    if client == McpClient::Opencode && was_empty {
        obj.insert(
            "$schema".to_string(),
            json!("https://opencode.ai/config.json"),
        );
    }

    existing
}

/// The per-harness server entry describing how to launch `gitid mcp serve`.
fn entry(client: McpClient, exe: &str) -> Value {
    match client {
        // OpenCode: a single command array, tagged local, explicitly enabled.
        McpClient::Opencode => json!({
            "type": "local",
            "command": [exe, "mcp", "serve"],
            "enabled": true,
        }),
        // Claude Code and Cursor share the command + args shape.
        McpClient::ClaudeCode | McpClient::Cursor => json!({
            "command": exe,
            "args": ["mcp", "serve"],
        }),
    }
}

/// The top-level key each harness stores its servers under.
fn servers_key(client: McpClient) -> &'static str {
    match client {
        McpClient::Opencode => "mcp",
        McpClient::ClaudeCode | McpClient::Cursor => "mcpServers",
    }
}

fn label(client: McpClient) -> &'static str {
    match client {
        McpClient::ClaudeCode => "Claude Code",
        McpClient::Cursor => "Cursor",
        McpClient::Opencode => "OpenCode",
    }
}

/// The config file a harness reads, global (user) by default or project-local.
fn config_path(ctx: &Ctx, client: McpClient, project: bool) -> PathBuf {
    if project {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match client {
            McpClient::ClaudeCode => cwd.join(".mcp.json"),
            McpClient::Cursor => cwd.join(".cursor").join("mcp.json"),
            McpClient::Opencode => cwd.join("opencode.json"),
        }
    } else {
        let home = &ctx.paths.home;
        match client {
            McpClient::ClaudeCode => home.join(".claude.json"),
            McpClient::Cursor => home.join(".cursor").join("mcp.json"),
            McpClient::Opencode => xdg_config_home(home).join("opencode").join("opencode.json"),
        }
    }
}

/// `$XDG_CONFIG_HOME`, or `~/.config` — where OpenCode's global config lives.
fn xdg_config_home(home: &Path) -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home.join(".config"))
}

fn read_json(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    serde_json::from_str(&text).with_context(|| format!("{} is not valid JSON", path.display()))
}

fn current_exe_string() -> Result<String> {
    let exe = std::env::current_exe().context("could not determine the gitid executable path")?;
    Ok(exe.to_string_lossy().into_owned())
}

fn confirm(prompt: &str) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        bail!("not a terminal; re-run with --yes to write, or --print to see the config");
    }
    inquire::Confirm::new(prompt)
        .with_default(false)
        .prompt()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_claude_code_config() {
        let out = render_config(
            Value::Object(Map::new()),
            McpClient::ClaudeCode,
            "/bin/gitid",
        );
        assert_eq!(out["mcpServers"]["gitid"]["command"], json!("/bin/gitid"));
        assert_eq!(out["mcpServers"]["gitid"]["args"], json!(["mcp", "serve"]));
        assert!(out.get("$schema").is_none());
    }

    #[test]
    fn fresh_opencode_config_has_schema_and_local_shape() {
        let out = render_config(Value::Object(Map::new()), McpClient::Opencode, "/bin/gitid");
        assert_eq!(out["$schema"], json!("https://opencode.ai/config.json"));
        assert_eq!(out["mcp"]["gitid"]["type"], json!("local"));
        assert_eq!(
            out["mcp"]["gitid"]["command"],
            json!(["/bin/gitid", "mcp", "serve"])
        );
        assert_eq!(out["mcp"]["gitid"]["enabled"], json!(true));
    }

    #[test]
    fn preserves_unrelated_keys_and_other_servers() {
        let existing = json!({
            "numFailedStartups": 3,
            "mcpServers": { "other": { "command": "other-server" } }
        });
        let out = render_config(existing, McpClient::Cursor, "/bin/gitid");
        // Unrelated top-level state survives.
        assert_eq!(out["numFailedStartups"], json!(3));
        // The pre-existing server survives alongside the new gitid entry.
        assert_eq!(out["mcpServers"]["other"]["command"], json!("other-server"));
        assert_eq!(out["mcpServers"]["gitid"]["command"], json!("/bin/gitid"));
    }

    #[test]
    fn does_not_add_schema_to_existing_opencode_file() {
        let existing = json!({ "model": "anthropic/claude" });
        let out = render_config(existing, McpClient::Opencode, "/bin/gitid");
        assert!(out.get("$schema").is_none());
        assert_eq!(out["model"], json!("anthropic/claude"));
        assert_eq!(out["mcp"]["gitid"]["type"], json!("local"));
    }

    #[test]
    fn idempotent_second_run_is_a_noop() {
        let once = render_config(
            Value::Object(Map::new()),
            McpClient::ClaudeCode,
            "/bin/gitid",
        );
        let twice = render_config(once.clone(), McpClient::ClaudeCode, "/bin/gitid");
        assert_eq!(once, twice);
    }
}
