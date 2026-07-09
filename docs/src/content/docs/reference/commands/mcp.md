---
title: gitid mcp
description: Run the gitid MCP server over stdio, or register it in agent-harness configs.
sidebar:
  order: 18
  label: mcp
---

Exposes gitid to AI agents over the [Model Context Protocol](https://modelcontextprotocol.io). Has two subcommands: `serve` runs the server, `install` registers it in a harness's config.

## Synopsis

```sh
gitid mcp serve
gitid mcp install [CLIENT]... [--project] [--print] [--yes]
```

## `gitid mcp serve`

Runs the MCP server over stdio (newline-delimited JSON-RPC 2.0) until the client closes the connection. This is a per-session child process — not a daemon — normally launched by an agent harness rather than by hand. Only protocol traffic is written to stdout; diagnostics go to stderr.

The server exposes read tools (`gitid_list`, `gitid_show`, `gitid_dirs`, `gitid_current`, `gitid_doctor`) and write tools (`gitid_use`, `gitid_forget`, `gitid_add`, `gitid_edit`, `gitid_remove`, `gitid_sync`). Every write flows through the same sync step as the CLI. See the [MCP server guide](../../../ai/mcp-server/) for the full tool list.

## `gitid mcp install`

Registers `gitid mcp serve` as an MCP server in one or more agent harnesses.

### Options

| Flag | Description |
| --- | --- |
| `[CLIENT]...` | Harnesses to configure: `claude-code`, `cursor`, `opencode`. Defaults to all of them when omitted. |
| `--project` | Write the project-local config instead of the global/user config. |
| `--print` | Print the config that would be written; modify nothing. |
| `-y, --yes` | Write without prompting. Required when stdin is not a terminal. |

### Config locations

| Client | Global config | Project config (`--project`) | Key |
| --- | --- | --- | --- |
| `claude-code` | `~/.claude.json` | `.mcp.json` | `mcpServers` |
| `cursor` | `~/.cursor/mcp.json` | `.cursor/mcp.json` | `mcpServers` |
| `opencode` | `~/.config/opencode/opencode.json` | `opencode.json` | `mcp` |

`install` merges the `gitid` entry into an existing config, preserving every other key, and writes atomically. It is idempotent — re-running when the entry is already present changes nothing. The entry launches the absolute path of the running `gitid` binary with `mcp serve`.

## Examples

Register with every supported harness (global):

```sh
gitid mcp install
```

Just Claude Code, project-local, without prompting:

```sh
gitid mcp install claude-code --project --yes
```

See what would be written for OpenCode without touching anything:

```console
$ gitid mcp install opencode --print
› OpenCode → /home/jane/.config/opencode/opencode.json
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

## See also

- [MCP server guide](../../../ai/mcp-server/) — setup, harnesses, and the tool catalog.
- [AI integration overview](../../../ai/overview/) — MCP vs. the Agent Skill.
