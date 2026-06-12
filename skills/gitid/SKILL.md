---
name: gitid
description: >-
  Manage and switch git identities per directory with the `gitid` CLI — set
  user.name/email, SSH key, commit signing, and isolated GitHub CLI auth, and
  assign a profile to a directory tree so every repo under it commits as the
  right person. Use when creating or switching git identities, fixing commits
  authored by the wrong user/email, isolating work vs personal SSH keys or gh
  logins, or onboarding a repo to an existing identity. LLM-agnostic: this is
  plain markdown usable by any agent.
license: MIT OR Apache-2.0
---

# gitid — per-directory git identities

`gitid` assigns a named **profile** (name, email, SSH key, signing, GitHub CLI
auth) to a **directory tree**. Every git repo under that tree then resolves the
right identity automatically — with no per-repo configuration.

## Mental model (read this first)

- gitid never edits a repository's `.git/config`. It manages git **conditional
  includes** (`includeIf "gitdir:…"`) in a file it owns, referenced once from
  the user's global gitconfig. Identity (name/email/signing/`core.sshCommand`)
  flows through git config; it needs **no shell or environment** to work.
- A **shell hook** carries the few things git config cannot: a per-profile
  `GH_CONFIG_DIR` (isolated `gh`/GitHub-CLI auth) and `GITID_PROFILE` for prompts.
- `gitid use <profile> <dir>` works **from anywhere** — you do NOT need to `cd`
  into the directory, and it does not touch any repo there.
- Two source-of-truth files live under `~/.config/gitid/`: `profiles.toml`
  (identities) and `mappings.toml` (directory→profile). Everything else is
  generated. After editing `profiles.toml` by hand, run `gitid sync`.

## Core workflow

```sh
# 1. Create a profile (non-interactive — preferred for agents/scripts)
gitid add work --non-interactive \
  --git-name "Jane Doe" --email jane@corp.example \
  --ssh-key ~/.ssh/id_work \
  --signing ssh --signing-key ~/.ssh/id_work.pub --sign-commits

# 2. Assign it to a directory tree (from anywhere)
gitid use work ~/code/work

# 3. Verify git actually resolves it
git -C ~/code/work/<any-repo> config user.email   # -> jane@corp.example
gitid current ~/code/work                          # gitid's view + cross-check
```

Always pass `--non-interactive` to `gitid add` in automation, or it will try to
open an interactive wizard and block.

## Recipes

**Set up a second (personal) identity and split a tree:**
```sh
gitid add personal --non-interactive --git-name "Jane" --email jane@home.example --ssh-key ~/.ssh/id_personal
gitid use personal ~/code            # broad default
gitid use work ~/code/work           # nested → longer path wins inside it
```

**Onboard the current repo to an existing profile** (no cd needed, but `.` works):
```sh
gitid use work .
```

**Switch a directory to a different profile:** just run `gitid use` again with the
new profile; the mapping is replaced.

**Inspect:**
```sh
gitid list            # profiles; ● marks the one active for your cwd
gitid dirs            # directory → profile mappings
gitid show work       # one profile's details + which dirs use it
gitid current --format name   # just the active profile name for cwd
```

**Remove:** `gitid remove work` (refuses if directories map to it; add `--force`
to drop those mappings too — the gh auth dir is left in place, never deleted).

**Recover after hand-editing `profiles.toml` or moving machines:** `gitid sync`.

## Diagnosing "commits show the wrong identity"

Run `gitid doctor [dir]` first — it checks git version, the global include,
whether generated files are current, mapped-dir existence, key files, gh auth,
and the hook. Then reason in this order:

1. **A local override.** `git -C <repo> config --show-origin user.email`. If the
   origin ends in `.git/config`, that repo set its own identity and it **beats**
   gitid. Fix: `git -C <repo> config --unset user.email` (and `user.name`).
2. **No mapping / wrong tree.** `gitid current <dir>` — if it says no profile is
   active, the directory isn't under any mapping. `gitid use <profile> <dir>`.
3. **Stale generated files.** `gitid doctor` says "out of date" → `gitid sync`.
4. **Missing global include.** doctor flags it → `gitid sync` re-adds it.
5. **Nested mappings:** the **longest** matching directory wins, both in gitid
   and in git. Map the specific subtree explicitly if needed.

## Shell hook (only needed for gh-auth isolation / prompt segment)

Identity works without it. Install once for `GH_CONFIG_DIR` switching:
`gitid setup` (detects the shell, asks before editing the rc file). `gitid hook
<shell>` prints the script; `gitid env --shell <shell>` is the hot path the hook
calls and must never be invoked for its side effects manually.

## When NOT to use gitid

- Don't hand-edit `~/.local/share/gitid/*` (generated; rerun `gitid sync`).
- Don't set identity in a repo's local config "to be safe" — it overrides gitid
  and reintroduces the problem gitid solves.
- Don't `export GIT_SSH_COMMAND` — it overrides every repo; gitid uses
  `core.sshCommand` per profile instead.

## More detail

- `reference.md` — every command, flag, the `profiles.toml`/`mappings.toml`
  schema, file layout, and environment variables.
- `troubleshooting.md` — symptom→cause→fix table and how the mechanism resolves.
