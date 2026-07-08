---
title: gitid init
description: Create gitid's config and data directories and bootstrap the global gitconfig include.
sidebar:
  order: 16
  label: init
---

Creates gitid's config and data directories, generates the derived files, and appends the single `[include]` line to your global gitconfig.

## Synopsis

```sh
gitid init
```

## Options

`gitid init` takes no options.

## What it does

1. Creates the config directory (`~/.config/gitid/`) and data directory (`~/.local/share/gitid/`), following XDG on Linux/macOS and Known Folders on Windows (`GITID_CONFIG_DIR` / `GITID_DATA_DIR` override them).
2. Runs a full [sync](../sync/): generates the include manifest and any profile fragments, and appends the `[include]` line to `~/.gitconfig` if it is not already there.
3. Prints a reminder to run [`gitid setup`](../setup/) to install the shell hook.

## When you need it

Usually never — it is optional. Every mutating command (`add`, `use`, `edit`, …) bootstraps lazily, creating the same directories and include line on first use. `gitid init` exists for when you want the bootstrap to happen explicitly and on its own, for example in a dotfiles or machine-provisioning script, or to verify a fresh install before creating any profiles.

## Examples

Explicit first-time bootstrap:

```console
$ gitid init
✔ initialised gitid (config: /home/jane/.config/gitid, data: /home/jane/.local/share/gitid)
✔ added gitid include to your global gitconfig
ℹ run `gitid setup` to install the shell hook
```

In a provisioning script, followed by the hook install:

```sh
gitid init
gitid setup zsh --yes
```

Running it again on an initialised machine is harmless — it re-syncs and reports nothing new.

## See also

- [`gitid sync`](../sync/) — the regeneration step init runs.
- [`gitid setup`](../setup/) — the follow-up step init suggests.
- [Environment variables](../../environment-variables/) — `GITID_CONFIG_DIR`, `GITID_DATA_DIR`.
