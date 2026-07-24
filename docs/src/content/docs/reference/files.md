---
title: Files
description: Every file gitid reads and writes — config dir, data dir, generated gitconfig fragments, and the single global include line.
---

gitid keeps a strict separation between two kinds of files:

- **Sources of truth** in the *config dir* — `profiles.toml` (hand-editable) and `mappings.toml` (machine-owned). Everything else is a pure function of these two files.
- **Generated artefacts** in the *data dir* — gitconfig fragments, the include manifest, gh config dirs, and the update-check cache. Never hand-edit these; run [`gitid sync`](../commands/sync/) to regenerate them.

The only file gitid touches outside its own directories is your global gitconfig, which gets exactly one appended `[include]` block.

## Locations

| Platform | Config dir | Data dir |
|---|---|---|
| Linux | `$XDG_CONFIG_HOME/gitid` (default `~/.config/gitid`) | `$XDG_DATA_HOME/gitid` (default `~/.local/share/gitid`) |
| macOS | `~/.config/gitid` (XDG, matching where git itself looks) | `~/.local/share/gitid` |
| Windows | `%APPDATA%\gitid` | `%APPDATA%\gitid` |

The environment variables `GITID_CONFIG_DIR` and `GITID_DATA_DIR` override these locations entirely — see [Environment variables](../environment-variables/).

## File tree

```
~/.config/gitid/                        # config dir — sources of truth
├── profiles.toml                       # identities; edit freely, then `gitid sync`
└── mappings.toml                       # directory → profile; machine-owned

~/.local/share/gitid/                   # data dir — generated, never hand-edit
├── include.gitconfig                   # manifest of includeIf directives
├── update-check.json                   # update-check throttle cache
├── profiles/
│   ├── personal.gitconfig              # per-profile gitconfig fragment
│   └── work.gitconfig
├── ssh/
│   └── work.pub                        # public key materialised from the ssh-agent
└── gh/
    ├── personal/                       # per-profile GH_CONFIG_DIR (gh tokens)
    └── work/

~/.gitconfig                            # yours — gets ONE appended [include] block
```

## Config dir

### `profiles.toml` — user-editable

The source of truth for identities. Fully documented in the [profiles.toml reference](../profiles-toml/). gitid edits it through a comment-preserving TOML editor, so hand-written comments and ordering survive `gitid add`/`gitid edit`. After hand edits, run `gitid sync`.

### `mappings.toml` — machine-owned

Directory-to-profile assignments, written by `gitid use` and `gitid forget`. Prefer those commands over editing it; if you do edit it, run `gitid sync` afterwards.

```toml
version = 1

[[mapping]]
dir = "/home/jane/code/work/"           # canonical: forward slashes, one trailing slash
profile = "work"
case_insensitive = false                # matched with gitdir/i: when true

# dir_literal appears only when the path you typed differed from the
# canonical (symlink-resolved) path; both get an includeIf line.
# dir_literal = "/home/jane/w/"

[mapping.env]                           # derived cache, refreshed by `gitid sync`;
                                        # read verbatim by the hook's `gitid env`
GH_CONFIG_DIR = "/home/jane/.local/share/gitid/gh/work"
```

Each mapping carries a derived `env` cache so the shell hook's hot path (`gitid env`) reads exactly one small file. `gitid sync` recomputes it from the profile's gh setting and `[env]` table.

## Data dir (generated)

:::caution
Everything under the data dir is managed by gitid and overwritten on every sync. Do not hand-edit; change `profiles.toml` or run `gitid use`, then [`gitid sync`](../commands/sync/).
:::

### `profiles/<name>.gitconfig` — per-profile fragment

One fragment per profile, rendered from `profiles.toml`. A profile with name, email, SSH key, and SSH commit signing generates:

```ini
# Managed by gitid. Do not edit; run `gitid sync` to regenerate.
[user]
	name = Jane Doe
	email = jane@corp.example
	signingkey = /home/jane/.ssh/id_work.pub
[gpg]
	format = ssh
[commit]
	gpgsign = true
[core]
	sshCommand = ssh -i /home/jane/.ssh/id_work -o IdentitiesOnly=yes
```

`~` paths from `profiles.toml` are expanded to absolute here (a quoted `~` inside `core.sshCommand` would not be shell-expanded). Fragments for profiles that no longer exist are pruned on sync.

### `include.gitconfig` — the manifest

One `includeIf` block per mapped directory, pointing at the profile fragment by a path *relative to the data dir*:

```ini
# Managed by gitid. Do not edit; run `gitid sync` to regenerate.
[includeIf "gitdir:~/code/"]
	path = profiles/personal.gitconfig
[includeIf "gitdir:~/code/work/"]
	path = profiles/work.gitconfig
```

Details of the generated patterns:

- Directories are normalised to forward slashes with a single trailing slash; git appends `**` to `gitdir:` patterns ending in `/`, so the include applies to every repo under the tree.
- A home-directory prefix is contracted to `~/` for portability.
- Case-insensitive mappings use `gitdir/i:` instead of `gitdir:`.
- Literal glob metacharacters (`*`, `?`, `[`) in paths are escaped.
- Blocks are ordered by ascending directory length, so a more specific (longer) directory comes last and wins under git's last-one-wins precedence.
- A mapping whose typed path differed from its canonical (symlink-resolved) path gets a second block for the literal path.

### `ssh/<name>.pub` — materialised agent keys

Written for every profile whose key lives in the ssh-agent (`ssh = { agent = "…" }` in `profiles.toml`). Sync resolves the selector against the running agent and writes the matching public key here; the profile's fragment points `core.sshCommand` (and, with `signing.key = "agent"`, `user.signingkey`) at this file. When the agent is unreachable, sync keeps the existing file and warns, so repos keep working from the cached copy. Files for profiles that no longer use an agent key are pruned. See [SSH keys](../../guides/ssh-keys/#keys-held-by-the-ssh-agent).

### `gh/<name>/` — isolated GitHub CLI config

Created by sync for every profile with gh isolation enabled. The shell hook exports `GH_CONFIG_DIR` pointing here while the profile is active, so each profile's `gh auth login` token lives in its own directory. `gitid remove` never deletes these (your tokens survive profile deletion).

### `update-check.json` — update throttle cache

Machine-owned cache for the opportunistic update check: the unix time of the last check and the newest release tag seen. Safe to delete; see [Environment variables](../environment-variables/) for the knobs that control the check.

## The global gitconfig include

gitid appends exactly one block to your global gitconfig — once, at end of file — and never touches the rest of it:

```ini
# Added by gitid.
[include]
	path = ~/.local/share/gitid/include.gitconfig
```

It resolves the global path the same way git does when writing: `$GIT_CONFIG_GLOBAL` if set, else `~/.gitconfig` if it exists, else `$XDG_CONFIG_HOME/git/config` if it exists, else `~/.gitconfig`.

:::note
The block is appended at end of file deliberately (rather than via `git config --add`, which would insert into the first existing `[include]` section). If a `[user]` section appeared *after* the include, it would override the profile fragments — appending at EOF keeps gitid's includes last, so mapped directories always win over your global identity.
:::

If the resolved global gitconfig is **read-only** — for example, managed by
Home-Manager or Nix, which symlink it into the store — gitid never clobbers it.
It writes the `[include]` block to a writable `.local` companion instead
(`~/.config/git/config.local`, or `~/.gitconfig.local` next to `~/.gitconfig`).
For git to actually load it, your managed config must include that companion:

```nix
# Home-Manager
programs.git.includes = [{ path = "~/.config/git/config.local"; }];
```

`gitid doctor` reports whether git actually loads the companion, and warns when
it does not.

Every mutating command bootstraps this include if it is missing; `gitid doctor` reports whether it is present.
