---
title: How it works
description: The two mechanisms behind gitid — git conditional includes for identity and a shell hook for gh isolation — and which files it owns.
---

gitid never edits your repositories. No repo's `.git/config` is ever touched, and assigning a profile to a directory works from anywhere — you don't need to `cd` there first. Instead, gitid leans on two mechanisms.

## Mechanism 1: git conditional includes

Git conditional includes carry the identity. gitid appends a **single** `[include]` line to your global gitconfig, pointing at a manifest file it owns:

```ini
# appended once, at the end of ~/.gitconfig
[include]
	path = ~/.local/share/gitid/include.gitconfig
```

That manifest contains one `includeIf "gitdir:…"` directive per mapped directory, each pointing at a generated per-profile fragment:

```ini
# ~/.local/share/gitid/include.gitconfig
# Managed by gitid. Do not edit; run `gitid sync` to regenerate.
[includeIf "gitdir:~/code/work/"]
	path = profiles/work.gitconfig
[includeIf "gitdir:~/code/oss/"]
	path = profiles/personal.gitconfig
```

Each fragment sets `user.name`, `user.email`, `core.sshCommand`, and signing keys for one profile:

```ini
# ~/.local/share/gitid/profiles/work.gitconfig (generated — do not edit)
[user]
	name = Jane Doe
	email = jane@corp.example
	signingkey = /home/jane/.ssh/id_work.pub
[core]
	sshCommand = ssh -i /home/jane/.ssh/id_work -o IdentitiesOnly=yes
[gpg]
	format = ssh
[commit]
	gpgsign = true
```

Git itself applies the right identity to every repo under a mapped tree — no per-repo configuration, no environment variables, no shell required. Nested mappings behave the way you'd expect: the longest matching directory wins, both in gitid and in git.

Your global gitconfig is touched exactly once (an append at the end of the file). Your hand-written config and comments are never rewritten. If it is read-only (e.g. managed by Home-Manager or Nix), gitid leaves it untouched and writes the include to a writable `.local` companion instead — see [Files](../../reference/files/#the-global-gitconfig-include).

## Mechanism 2: the shell hook

A shell hook carries everything git config *cannot*:

- an isolated `GH_CONFIG_DIR`, so each profile has its own GitHub CLI (`gh`) auth
- a `GITID_PROFILE` environment variable for your prompt
- any extra variables you declare in a profile's `[env]` table

The hook runs on directory change, resolves the directory→profile mapping, and exports (or restores) the right variables. It is cheap: it only does work when the directory actually changes, and prints nothing when the active profile is unchanged.

:::note
The hook is **optional**. Identity — name, email, SSH key, signing — flows entirely through git config and works with no shell integration at all. Install the hook only if you want per-profile `gh` auth or a prompt segment. See [Shell integration](../../guides/shell-integration/).
:::

## What "identity resolution" means

When gitid (or git) resolves an identity for a directory, it finds the mapping whose path is the longest prefix of that directory and applies that profile. `gitid current [dir]` shows gitid's view and, when the directory is inside a repo, cross-checks what `git config user.email` actually returns — so it catches the one thing that can still override gitid: an identity set in a repo's own `.git/config`, which always beats any include. `gitid doctor` reports the same, plus stale files, missing keys, and hook problems.

## File layout and ownership

```
~/.config/gitid/
├── profiles.toml        # SOURCE OF TRUTH — edit freely, then run `gitid sync`
└── mappings.toml        # directory → profile assignments (machine-owned;
                         #   prefer `gitid use` / `gitid forget`)

~/.local/share/gitid/
├── include.gitconfig    # GENERATED manifest of includeIf directives — never edit
├── profiles/
│   └── <name>.gitconfig # GENERATED per-profile fragment — never edit
└── gh/
    └── <name>/          # per-profile GH_CONFIG_DIR (gh tokens live here)

~/.gitconfig             # yours — gets ONE appended [include] line, once
```

The ownership rules:

- **`profiles.toml`** is yours. Hand-edit it whenever you like, then run `gitid sync` to regenerate everything derived from it.
- **`mappings.toml`** is machine-owned. Use `gitid use` and `gitid forget` instead of editing it; if you do edit it, run `gitid sync` afterwards.
- **Everything under `~/.local/share/gitid/`** is generated. Never hand-edit these files — any change is overwritten on the next sync, and `add`, `edit`, `use`, `remove`, and `forget` all sync automatically.

Paths follow XDG on Linux/macOS and Known Folders on Windows. Override the locations with `GITID_CONFIG_DIR` and `GITID_DATA_DIR`.

:::note[Requirements]
gitid requires **git ≥ 2.13** (the first version with `includeIf` conditional includes). Windows support is best-effort: a PowerShell hook is provided, but CI covers Linux and macOS.
:::
