---
title: Using gitid with AI agents
description: gitid ships a portable Agent Skill that teaches any LLM agent the mental model, core workflow, and diagnosis order for driving gitid.
---

gitid ships a portable, LLM-agnostic **Agent Skill** in [`skills/gitid/`](https://github.com/dgreco-at-speer/gitid/tree/main/skills). It is plain markdown — no framework or model lock-in — and consists of:

- `SKILL.md` — the mental model (identity flows through git conditional includes; never edit a repo's local config), the core workflow, and the diagnosis order for "commits show the wrong identity"
- `reference.md` — every command and flag, the `profiles.toml`/`mappings.toml` schema, file layout, and environment variables
- `troubleshooting.md` — a symptom → cause → fix table

## Pointing an agent at the skill

**Claude Code / Claude Agent SDK / claude.ai** follow the Agent Skills format directly. Make the skill discoverable by symlinking (or copying) it:

```sh
mkdir -p ~/.claude/skills
ln -s "$PWD/skills/gitid" ~/.claude/skills/gitid
```

The agent loads it on demand based on the `description` in the skill's frontmatter.

**Other coding agents** (Cursor, Codex, Continue, …): point the agent at `skills/gitid/SKILL.md`, or reference it from your `AGENTS.md` / rules file.

**Any chat LLM**: paste the contents of `SKILL.md` (and `reference.md` if you need full flag detail) into the conversation before asking it to run gitid.

## Non-interactive usage

Agents and scripts must avoid the interactive wizard: `gitid add` without flags opens prompts and will block (or hang) a non-TTY session. Pass `--non-interactive`, which makes `gitid add` **fail with an error rather than prompt** for anything missing:

```sh
gitid add work --non-interactive \
  --git-name "Jane Doe" --email jane@corp.example \
  --ssh-key ~/.ssh/id_work \
  --signing ssh --signing-key ~/.ssh/id_work.pub --sign-commits
```

In non-interactive mode, `--git-name` and `--email` are required, and `--signing-key` is required whenever `--signing` is `ssh` or `openpgp`. GitHub CLI isolation is provisioned by default; pass `--no-gh` to skip it.

The rest of the surface is already non-interactive and machine-friendly:

```sh
gitid use work ~/code/work           # no prompts
gitid current --format json          # machine-readable resolution
gitid current --quiet                # exit 0/1: is a profile active here?
gitid list --format json
gitid setup --yes                    # install the hook without prompting
```

:::tip
The skill also encodes what agents should *not* do — chiefly, never "fix" an identity by writing `user.email` into a repo's local config, which overrides gitid and reintroduces the original problem. See [Common issues](../../troubleshooting/common-issues/) for the same guidance in human form.
:::
