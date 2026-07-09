---
title: MCP server
description: gitid's built-in Model Context Protocol server exposes profiles and directory mappings to AI agents as typed tools over stdio.
---

gitid includes a [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server so agents can manage git identities through structured tools instead of shelling out and parsing text output.

It is a **stdio server**: the agent harness launches `gitid mcp serve` as a child process for the duration of a session and talks to it over stdin/stdout. There is no daemon, socket, or background service to manage — nothing runs when no agent is connected.

## Quick setup

Register gitid with every supported harness, at the user (global) level:

```sh
gitid mcp install
```

Or name specific harnesses:

```sh
gitid mcp install claude-code cursor opencode
```

`install` merges a `gitid` entry into each harness's MCP config, **preserving everything else in the file**, and prompts before writing. Restart (or reload) the harness afterward so it picks up the new server.

## Supported harnesses

| Harness | Global config | Project config (`--project`) | Key |
| --- | --- | --- | --- |
| `claude-code` | `~/.claude.json` | `.mcp.json` | `mcpServers` |
| `cursor` | `~/.cursor/mcp.json` | `.cursor/mcp.json` | `mcpServers` |
| `opencode` | `~/.config/opencode/opencode.json` | `opencode.json` | `mcp` |

By default `install` writes the **global** (user-level) config so gitid is available in every project. Pass `--project` to write the repo-local config instead — useful for committing MCP setup into a shared repo.

Each entry points at the absolute path of the running `gitid` binary (so the harness can launch it regardless of `PATH`) with the arguments `mcp serve`.

## Tools

The server exposes gitid's operations as tools. Read tools are safe and idempotent; write tools change configuration and run the same sync step as the CLI, regenerating all derived git config.

| Tool | Kind | Purpose |
| --- | --- | --- |
| `gitid_list` | read | List all profiles |
| `gitid_show` | read | Show one profile's details |
| `gitid_dirs` | read | List directory → profile mappings |
| `gitid_current` | read | Which profile is active for a directory |
| `gitid_doctor` | read | Diagnose configuration problems |
| `gitid_use` | write | Assign a profile to a directory tree |
| `gitid_forget` | write | Remove a directory mapping |
| `gitid_add` | write | Create a profile (non-interactive) |
| `gitid_edit` | write | Modify a profile's fields |
| `gitid_remove` | write | Delete a profile |
| `gitid_sync` | write | Regenerate derived files from the stores |

`gitid_add` is non-interactive by construction: pass `name`, `git_name`, and `email` (and `signing`/`signing_key` when signing commits). There is no wizard and no prompting — a missing required field is returned as an error.

Every write tool returns the resulting state plus a summary of what sync changed, so the agent can confirm the effect in one round-trip.

## Inspecting the config

To see exactly what would be written without touching anything:

```sh
gitid mcp install opencode --print
```

```json
{
  "mcp": {
    "gitid": {
      "type": "local",
      "command": ["/home/jane/.local/bin/gitid", "mcp", "serve"],
      "enabled": true
    }
  }
}
```

Registering again is a no-op — if the `gitid` entry is already present and identical, `install` reports it and changes nothing.

## Running the server directly

You normally never run this yourself — the harness does. But it can be invoked manually (for debugging) and speaks newline-delimited JSON-RPC 2.0 over stdio:

```sh
gitid mcp serve
```

The server writes only protocol traffic to stdout; diagnostics go to stderr. It runs until the client closes the connection.

## See also

- [`gitid mcp`](../../reference/commands/mcp/) — command reference.
- [Agent Skill](../agent-skills/) — the portable, markdown alternative (or complement).
- [AI integration overview](../overview/) — MCP vs. skill, and when to use each.
