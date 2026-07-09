---
title: AI integration
description: Two ways to let AI agents drive gitid — a native MCP server and a portable Agent Skill — and when to use each.
---

gitid is built to be driven by AI coding agents, not just humans. Agents routinely work across repositories that need different identities (work vs. personal, client A vs. client B), and gitid is how they set the right one without touching any repo's config.

There are two integration paths. They are complementary — use either or both.

## MCP server (native tools)

gitid ships a [Model Context Protocol](https://modelcontextprotocol.io) server. The agent's harness spawns `gitid mcp serve` and calls gitid operations as **typed tools** (`gitid_list`, `gitid_use`, `gitid_add`, …) — no shell parsing, no guessing at flags.

Best when your harness speaks MCP (Claude Code, Cursor, OpenCode, …). One command wires it up globally:

```sh
gitid mcp install
```

See [MCP server](../mcp-server/).

## Agent Skill (portable instructions)

gitid also ships a portable, LLM-agnostic **Agent Skill**: plain markdown teaching any agent the mental model, the core workflow, and how to diagnose "commits show the wrong identity". It works with any agent that can read instructions — including ones without MCP support — and can even be pasted into a chat.

See [Agent Skill](../agent-skills/).

## Which should I use?

| | MCP server | Agent Skill |
| --- | --- | --- |
| Integration | Typed tools over stdio | Markdown instructions |
| Requires | An MCP-capable harness | Any agent that reads text |
| Teaches the *why* | Tool descriptions only | Full mental model + diagnosis order |
| Setup | `gitid mcp install` | Symlink into the agent's skills dir |

They pair well: install the **skill** so the agent understands gitid's model and pitfalls, and the **MCP server** so it can act through structured tools. Both keep the golden rule — never "fix" an identity by writing `user.email` into a repo's local config.

:::note
Whichever path you use, agents must stay non-interactive. `gitid add` without flags opens a wizard that will block a non-TTY session; the MCP `gitid_add` tool is non-interactive by construction, and the CLI takes `--non-interactive`. See [Agent Skill → Non-interactive usage](../agent-skills/#non-interactive-usage).
:::
