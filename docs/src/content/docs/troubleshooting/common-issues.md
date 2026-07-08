---
title: Common issues
description: Start with gitid doctor, then use the symptom-to-fix table for wrong identities, hook problems, gh auth, SSH keys, and stale files.
---

## Start with `gitid doctor`

`gitid doctor [dir]` runs most of the checks below in one shot and prints a concrete fix for each problem it finds (exit code 0 = healthy, 1 = problems). It checks:

- **git itself** — installed and ≥ 2.13 (the first version with `includeIf`)
- **the global include** — your global gitconfig actually includes gitid's manifest
- **generated files** — every profile fragment and the includeIf manifest match what `gitid sync` would generate (i.e. nothing is stale after hand-edits)
- **mappings** — every mapping points at an existing profile, and mapped directories exist on disk
- **live precedence probe** — if the inspected directory is inside a repo, doctor asks git what `user.email` actually resolves there and warns (with the overriding file's origin) when it differs from the mapped profile
- **SSH keys** — each profile's SSH key and SSH signing key exist, and key permissions are not group/world readable
- **gh** — if any profile enables gh isolation: the `gh` CLI is installed, and each such profile has authenticated (with the exact `GH_CONFIG_DIR=… gh auth login` command when it hasn't)
- **the shell hook** — a `gitid hook` line is present in `~/.bashrc`, `~/.zshrc`, or fish's config

```console
$ gitid doctor ~/code/work/api
✓ git 2.49
✓ global gitconfig includes gitid manifest
✓ generated files are up to date
✓ git resolves jane@corp.example in this repo
✓ work: gh auth configured
✓ shell hook installed

all checks passed
```

## Symptom → cause → fix

| Symptom | Likely cause | Fix |
|---|---|---|
| Commits show the wrong name/email in a repo | The repo has a **local** `user.*` in its `.git/config`, which overrides gitid | `git -C <repo> config --show-origin user.email`; if the origin is `.git/config`, run `git -C <repo> config --unset user.email` (and `user.name`) |
| Identity not applied anywhere under a tree | No mapping for that tree | `gitid current <dir>` to confirm, then `gitid use <profile> <dir>` |
| `gitid use` succeeded but git still shows nothing | Generated files stale, or the global include is missing | `gitid sync`, then verify with `git -C <repo> config --show-origin --get user.email` |
| Wrong profile inside a nested directory | A broader mapping is matching | The **longest** matching directory wins — map the subtree explicitly: `gitid use <profile> <specific-subdir>` |
| `gitid add` hangs in a script or agent | The interactive wizard is waiting on input | Re-run with `gitid add <name> --non-interactive --git-name … --email …` |
| `gh` uses the wrong account | Hook not installed, so `GH_CONFIG_DIR` isn't exported | `gitid setup`, restart the shell; check `echo $GH_CONFIG_DIR` inside a mapped dir |
| `GH_CONFIG_DIR` empty in a mapped dir | Hook not loaded in this shell, or the profile has `gh.enabled = false` | Confirm the rc line and restart the shell; `gitid show <profile>` to check the gh setting |
| Pushes use the wrong SSH key | An agent offered another key first, or something overrides `core.sshCommand` | gitid already sets `core.sshCommand … -o IdentitiesOnly=yes`; make sure no repo-local `core.sshCommand` and no exported `GIT_SSH_COMMAND` override it |
| A symlinked path doesn't match | gitid stores the canonical path | gitid also emits the literal path you typed; if it still misses, map the resolved path: `gitid use <profile> <real-path>` |
| A linked worktree ignores the profile | Its main repo lives outside the mapped tree (git matches the **main** repo's git dir) | Map the main repo's directory: `gitid use <profile> <main-repo-dir>` |
| Edited `profiles.toml`, nothing changed | Derived files not regenerated | `gitid sync` |
| Everything broke after a machine move | Generated files and the global include aren't present on the new machine | `gitid sync` — it re-adds the global include and regenerates all fragments |

## Rules of thumb

- **Never** "fix" an identity by setting `user.email` in a repo's local config — it overrides gitid permanently and reintroduces the problem gitid solves. Unset local overrides instead.
- **Never** hand-edit anything under `~/.local/share/gitid/` — it's generated; `gitid sync` overwrites it.
- **Never** `export GIT_SSH_COMMAND` — the environment variable overrides `core.sshCommand` for every repo in the shell, defeating per-profile keys.
- After any hand-edit of `~/.config/gitid/profiles.toml` or `mappings.toml`, run `gitid sync`. (`add`, `edit`, `use`, `remove`, and `forget` sync automatically.)

If the table doesn't cover your case, walk the resolution chain link by link — see [How resolution works](../how-resolution-works/). For hook installation details, see [Shell integration](../../guides/shell-integration/); for gh specifics, see [Per-profile GitHub CLI auth](../../guides/github-cli/).
