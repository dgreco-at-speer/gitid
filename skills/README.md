# gitid agent skills

Portable, LLM-agnostic knowledge that teaches an AI agent how to operate `gitid`.
Each skill is a folder with a `SKILL.md` (YAML frontmatter + markdown) plus
optional reference files — plain text, no framework or model lock-in.

## Skills

- [`gitid/`](gitid/SKILL.md) — manage and switch git identities per directory:
  create profiles, assign them to directory trees, verify, and troubleshoot
  "wrong author/email" problems. Supporting docs: `reference.md` (full command
  and schema reference) and `troubleshooting.md` (symptom→fix table).

## Using these skills with any agent

- **Claude Code / Claude Agent SDK / claude.ai:** these follow the Agent Skills
  format directly. Make the skill discoverable, e.g. symlink or copy it:
  ```sh
  mkdir -p ~/.claude/skills
  ln -s "$PWD/skills/gitid" ~/.claude/skills/gitid
  ```
  The agent loads it on demand based on the `description` in the frontmatter.
- **Other coding agents (Cursor, Codex, Continue, …):** point the agent at
  `skills/gitid/SKILL.md`, or reference it from your `AGENTS.md` / rules file.
- **Any chat LLM:** paste the contents of `SKILL.md` (and `reference.md` if you
  need full flag detail) into the conversation before asking it to run `gitid`.

The body is the contract: it tells the agent the mental model (identity flows
through git conditional includes; never edit a repo's local config), the core
workflow, and the diagnosis order. The frontmatter `description` is what an agent
matches on to decide the skill is relevant.
