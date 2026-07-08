---
title: How resolution works
description: The full identity-resolution chain, git's precedence rules, nested and symlinked mappings, worktree behavior, and how to inspect every link manually.
---

When `gitid current` and `git config user.email` disagree — or an identity mysteriously doesn't apply — you need to know exactly how a directory turns into an identity. There are two independent chains: git config (identity) and the shell hook (environment).

## The git config chain (identity)

1. **Global gitconfig → gitid's manifest.** Your global gitconfig contains exactly one line gitid ever wrote to it:

   ```ini
   [include]
   	path = ~/.local/share/gitid/include.gitconfig
   ```

2. **Manifest → per-mapping `includeIf`.** The generated manifest holds one `includeIf "gitdir:…"` block per mapping (two for a symlinked tree — see below), each pointing at a profile fragment:

   ```ini
   # Managed by gitid. Do not edit; run `gitid sync` to regenerate.
   [includeIf "gitdir:~/code/"]
   	path = profiles/personal.gitconfig
   [includeIf "gitdir:~/code/work/"]
   	path = profiles/work.gitconfig
   ```

   Blocks are ordered ascending by directory length. In git config, **later includes win**, so the longer (more specific) directory takes effect inside nested mappings.

3. **`gitdir:` match.** Inside a repo, git matches the repo's **git dir** against each pattern and applies the matching fragments. Case-insensitive mappings use `gitdir/i:` instead (the default on Windows/macOS).

4. **Fragment → identity.** The matched fragment sets `user.name`, `user.email`, `core.sshCommand`, `gpg.format`, `user.signingkey`, `commit.gpgsign`, and any `[extra]` passthrough keys.

No shell, no environment variables, no per-repo config — git does all of this itself, which is why identity works in GUIs, IDEs, and cron jobs too.

## Precedence: repo-local always wins

Git applies config in order, last value wins:

```
system  <  global (where gitid's include lives)  <  repo-local (.git/config)
```

A `user.email` set in a repo's own `.git/config` therefore beats gitid **every time**. This is the #1 cause of "wrong identity" surprises. Both `gitid current` and `gitid doctor` detect it by asking git what actually resolves and comparing against the mapped profile:

```console
$ gitid current ~/code/work/api
profile: work
  name:  Jane Doe
  email: jane@corp.example
! git resolves user.email = old@example.com here (from file:.git/config), not the profile's jane@corp.example; a local override may be set
```

Fix: `git -C <repo> config --unset user.email` (and `user.name`).

## Nested mappings: longest path wins

Both gitid's own resolution (`gitid current`, the shell hook) and git's include ordering agree: the mapping with the **longest** matching directory wins.

```sh
gitid use personal ~/code            # broad default
gitid use work ~/code/work           # more specific → wins inside ~/code/work
```

## Symlinked paths

`gitid use` canonicalizes the directory you give it and stores the canonical path. If the path you typed differs (a symlink), gitid emits `includeIf` blocks for **both** the canonical and the literal path, so the mapping matches however the tree is reached. If a differently-named symlink into the tree still misses, map the resolved real path explicitly.

## Worktree caveat

Git matches `gitdir:` patterns against a repo's git dir, and a linked worktree's git dir lives under the **main** repository. So a worktree located inside a mapped tree whose main repo is *outside* it will not pick up the profile. Fix: map the main repo's directory (`gitid use <profile> <main-repo-dir>`).

## The shell hook chain (gh auth and prompt)

Everything above needs no shell. The hook covers only what git config cannot carry:

1. Your rc file evals `gitid hook <shell>`, which installs a directory-change handler.
2. On every directory change, the handler runs `gitid env --shell <shell>` — the hot path. It matches your cwd against `mappings.toml` (longest path wins, same rule).
3. `gitid env` prints export/unset statements for `GITID_PROFILE`, `GH_CONFIG_DIR` (if the profile enables gh), and any profile `[env]` vars — plus `GITID_STATE`, an internal variable recording the pre-activation values so leaving the tree restores them exactly. It prints **nothing** when the active profile is unchanged.

If `gh` or your prompt is wrong while `git config user.email` is right, the problem is in this chain (hook not installed or not loaded in this shell), not in git config. Setting `GITID_DISABLE` to a non-empty value makes `gitid env` emit nothing — a kill switch worth checking if activation silently stopped.

## Inspecting each link manually

Work down the chain until a link is broken:

```sh
# What does git actually resolve, and from which file?
git -C <repo> config --show-origin --get user.email

# Link 1: is gitid's include present in the global config?
git config --global --get-all include.path      # should list …/gitid/include.gitconfig

# Link 2: does the manifest have an includeIf for your directory?
cat ~/.local/share/gitid/include.gitconfig

# Link 4: does the fragment say what you expect?
cat ~/.local/share/gitid/profiles/<name>.gitconfig

# gitid's own view, plus the cross-check warning on mismatch
gitid current <dir>

# Everything at once, with fix hints
gitid doctor <dir>
```

Interpreting a `gitid current` vs `git config` disagreement:

- **`gitid current` names a profile, git resolves a different email** → look at the `--show-origin` output. Origin ends in `.git/config`: local override, unset it. Origin is some other global file: config set after gitid's include (later wins), remove or move it.
- **`gitid current` names a profile, git resolves nothing** → the manifest or fragment is stale or the global include is missing; `gitid sync` regenerates and re-adds all of it. If it persists, you may be in a linked worktree (see above).
- **`gitid current` says no profile active** → the directory isn't under any mapping (`gitid dirs` to list them); `gitid use <profile> <dir>`.

## Requirements

The whole mechanism needs **git ≥ 2.13** (`includeIf` with `gitdir:` and `gitdir/i:`). `gitid doctor` checks your git version first for exactly this reason.
