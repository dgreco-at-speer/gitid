# gitid troubleshooting

Start with `gitid doctor [dir]` — it runs most of these checks and prints fixes.

## Symptom → cause → fix

| Symptom | Likely cause | Fix |
|---|---|---|
| Commits show the wrong name/email in a repo | The repo has a **local** `user.*` in its `.git/config`, which overrides gitid | `git -C <repo> config --show-origin user.email`; if origin is `.git/config`, `git -C <repo> config --unset user.email` (and `user.name`) |
| Identity not applied anywhere under a tree | No mapping for that tree | `gitid current <dir>` to confirm; `gitid use <profile> <dir>` |
| `gitid use` succeeded but git still shows nothing | Generated files stale, or global include missing | `gitid sync`; verify with `git -C <repo> config --show-origin --get user.email` |
| Wrong profile inside a nested directory | A broader mapping is matching | The **longest** matching directory wins — `gitid use <profile> <specific-subdir>` |
| `gitid add` hangs | Interactive wizard waiting on input | Re-run with `--non-interactive --git-name … --email …` |
| `gh` uses the wrong account | Hook not installed, so `GH_CONFIG_DIR` isn't exported | `gitid setup`, restart the shell; check `echo $GH_CONFIG_DIR` inside a mapped dir |
| `GH_CONFIG_DIR` empty in a mapped dir | Hook not loaded in this shell, or profile has `gh.enabled = false` | Confirm the rc line, restart shell; `gitid show <profile>` |
| Pushes use the wrong SSH key | Agent offered another key first | gitid sets `core.sshCommand … -o IdentitiesOnly=yes`; ensure no repo-local `core.sshCommand` and no exported `GIT_SSH_COMMAND` override it |
| Symlinked path doesn't match | gitid stores the canonical path | gitid also emits the literal path; if it still misses, `gitid use <profile> <real-path>` |
| Linked worktree ignores the profile | Its main repo lives outside the mapped tree | Map the main repo's directory: `gitid use <profile> <main-repo-dir>` |
| Edited `profiles.toml`, nothing changed | Derived files not regenerated | `gitid sync` |
| Everything broke after a machine move | Generated paths/include not present | `gitid sync` (re-adds the global include and regenerates fragments) |

## How resolution actually works (for deeper debugging)

1. The user's global gitconfig has one `[include] path = …/include.gitconfig`.
2. That manifest has `[includeIf "gitdir:<dir>/"] path = profiles/<name>.gitconfig`
   per mapping, ordered so longer (more specific) directories come last and win.
3. Git, inside any repo, matches its git-dir against those patterns and applies
   the matching fragment's `user.*`, `gpg.*`, `commit.*`, `core.sshCommand`.
4. Precedence (last wins): system < global (gitid include) < **repo-local**. So a
   repo-local `user.email` always beats gitid — this is the #1 cause of surprises.

Verify the whole chain with:
```sh
git -C <repo> config --show-origin --get user.email
git config --global --get-all include.path     # should list gitid's include
gitid doctor <repo>
```

## Requirements

git ≥ 2.13 (for `includeIf` / `gitdir/i`). gitid is cross-platform; Linux/macOS
are first-class, Windows is best-effort.
