---
title: gitid sync
description: Regenerate all derived files from profiles.toml and mappings.toml.
sidebar:
  order: 11
  label: sync
---

Regenerates every derived file from the two sources of truth (`profiles.toml` and `mappings.toml`), so the on-disk state is always a pure function of them.

## Synopsis

```sh
gitid sync
```

## Options

`gitid sync` takes no options.

## What it writes

| File | Action |
| --- | --- |
| `~/.local/share/gitid/profiles/<name>.gitconfig` | One git-config fragment per profile (`user.name`, `user.email`, `core.sshCommand`, signing keys, extras); fragments for deleted profiles are pruned. |
| `~/.local/share/gitid/include.gitconfig` | The manifest of `includeIf "gitdir:…"` directives, one per directory mapping. |
| `~/.local/share/gitid/ssh/<name>.pub` | For profiles with an agent-held key (`ssh = { agent = "…" }`): the public key resolved from the running ssh-agent; stale files are pruned. |
| `~/.config/gitid/mappings.toml` | Each mapping's cached `[mapping.env]` table is refreshed from its profile (e.g. `GH_CONFIG_DIR`, custom `[profiles.<name>.env]` vars). |
| `~/.local/share/gitid/gh/<name>/` | Created for each profile with gh isolation enabled. |
| `~/.gitconfig` | The single `[include]` line pointing at the manifest is appended if missing (once, at end of file). |

Writes are idempotent: files that already match are left untouched, and the summary reports only real changes.

Resolving agent-held keys is the one step that needs something running: when the ssh-agent is unreachable or a selector no longer matches, sync keeps the previously materialised `.pub` file and warns instead of failing. It only errors when the key was never materialised at all (the generated config would point at a missing file).

## When to run it

Run `gitid sync` after **hand-editing** `profiles.toml` (or `mappings.toml`) — the generated git config does not change until you do. You do not need it after normal commands: `add`, `edit`, `remove`, `use`, and `forget` all sync automatically.

## Examples

After editing a profile's email by hand:

```console
$ vi ~/.config/gitid/profiles.toml
$ gitid sync
✓ synced: 1 fragment(s) written, include regenerated
```

When everything is already up to date, sync says nothing and exits 0:

```console
$ gitid sync
$
```

If a mapping points at a profile you renamed or deleted, sync warns instead of guessing:

```console
$ gitid sync
⚠ mapping references unknown profile "wrok"; run `gitid use` or remove it
```

## See also

- [`gitid doctor`](../doctor/) — detects when the generated files are stale.
- [`gitid init`](../init/) — first-time bootstrap (creates the directories, then syncs).
