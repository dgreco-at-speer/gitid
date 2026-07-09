---
title: gitid update
description: Update gitid to the latest release, or check whether one exists.
sidebar:
  order: 17
  label: update
---

Downloads a gitid release and replaces the running binary in place — it swaps whichever `gitid` is executing, wherever it lives.

## Synopsis

```sh
gitid update [--check] [--force] [--version <TAG>]
```

## Options

| Flag | Description |
| --- | --- |
| `--check` | Only check whether a newer version exists; do not install anything. |
| `-f, --force` | Reinstall even if already up to date (or already at the requested version). |
| `--version <TAG>` | Install a specific release tag instead of the latest. Defaults to `$GITID_VERSION` when that is set. |

## Behavior

- Without flags, `update` resolves the latest release tag, and if it is newer than the running version, downloads the binary for your platform and self-replaces. If you are already up to date it says so and exits without downloading (use `--force` to reinstall anyway).
- With `--version` (or `$GITID_VERSION`), the given tag is installed if it differs from the running version — including downgrades.
- On platforms with no prebuilt binary (currently macOS), `update` fails with a `cargo install --git …` hint instead.

## Background update check

gitid also checks for updates opportunistically, so you hear about new releases without ever running `update`:

- After a successful interactive command, it reads a **cached** result and, if a newer version is known, prints a one-line notice to **stderr** — it never installs anything on its own and never blocks on the network.
- When the cache is stale (older than 24 hours by default; tune with `GITID_UPDATE_INTERVAL`, in seconds), it spawns a detached `gitid update --refresh-cache` worker to refresh it for next time.
- It only probes the network from interactive sessions, and stays completely silent for shell-eval'd and machine-facing commands (`env`, `hook`, `completions`, `current`) so prompts and pipelines are never disturbed.
- Set `GITID_NO_UPDATE_CHECK` to any non-empty value to disable it entirely.

See the [self-update guide](../../../guides/self-update/) for details, including authentication for private release repos.

## Examples

Check without installing:

```console
$ gitid update --check
› gitid 0.4.0 is available (you have %GITID_VERSION%). Run `gitid update`.
```

Install the latest release:

```console
$ gitid update
✓ updated gitid to 0.4.0
```

Pin a specific tag (e.g. to roll back):

```sh
gitid update --version v0.2.0
```

## Notes

- Releases in a private repository need auth: install the [GitHub CLI](https://cli.github.com) and `gh auth login`, or set `GH_TOKEN`.
- After updating, regenerate your [shell completions](../completions/) to pick up any new flags.

## See also

- [Self-update guide](../../../guides/self-update/)
- [Environment variables](../../environment-variables/) — `GITID_VERSION`, `GITID_NO_UPDATE_CHECK`, `GITID_UPDATE_INTERVAL`.
