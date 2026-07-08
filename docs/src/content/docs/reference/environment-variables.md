---
title: Environment variables
description: Every environment variable gitid reads or sets — overrides, kill switches, update knobs, and the variables the shell hook exports.
---

Two directions to keep apart: variables **you set** to change gitid's behaviour, and variables **the shell hook sets** in your session when a profile activates.

## Summary

| Variable | Direction | Purpose |
|---|---|---|
| `GITID_CONFIG_DIR` | you set | Override the config directory. |
| `GITID_DATA_DIR` | you set | Override the data directory. |
| `GITID_DISABLE` | you set | Kill switch: `gitid env` emits nothing. |
| `GITID_NO_UPDATE_CHECK` | you set | Disable the opportunistic update check. |
| `GITID_UPDATE_INTERVAL` | you set | Update-check throttle interval, in seconds. |
| `GITID_REPO` | you set | GitHub `owner/repo` used by `gitid update` and the installer. |
| `GITID_VERSION` | you set | Pin the release tag for `gitid update` and the installer. |
| `GH_TOKEN` / `GITHUB_TOKEN` | you set | Auth for release downloads from a private repo. |
| `GITID_PROFILE` | gitid sets | Active profile name (for prompts). |
| `GH_CONFIG_DIR` | gitid sets | Isolated GitHub CLI config dir for the active profile. |
| `GITID_STATE` | gitid sets | Internal saved-values diff. Do not set manually. |

## Variables you set

### `GITID_CONFIG_DIR`

Overrides the config directory (default: platform config dir + `gitid`, e.g. `~/.config/gitid`). Holds `profiles.toml` and `mappings.toml`. Intended for the test suite and custom installs. See [Files](../files/).

### `GITID_DATA_DIR`

Overrides the data directory (default: platform data dir + `gitid`, e.g. `~/.local/share/gitid`). Holds all generated artefacts: `include.gitconfig`, `profiles/*.gitconfig`, `gh/<name>/`, and `update-check.json`.

:::caution
If you change either override after mapping directories, the paths baked into your global gitconfig include and the mapping env caches still point at the old locations — run `gitid sync` under the new environment and check `gitid doctor`.
:::

### `GITID_DISABLE`

When set to any non-empty value, `gitid env` (the shell-hook hot path) prints nothing at all — no activation, no deactivation. A kill switch for the hook without editing your shell rc file.

### `GITID_NO_UPDATE_CHECK`

When set to any non-empty value, disables the opportunistic background update check entirely: no notice is printed and no background refresh is spawned. Explicit `gitid update` and `gitid update --check` still work.

### `GITID_UPDATE_INTERVAL`

Throttle interval for the opportunistic update check, in **seconds**. Default: `86400` (24 hours). The check reads a cached result and only spawns a detached network refresh when the cache is older than this interval — and only in interactive sessions (stderr is a terminal). Non-numeric values fall back to the default.

### `GITID_REPO`

The GitHub `owner/repo` that `gitid update` queries for releases. Also read by the install scripts. Defaults to the upstream gitid repository.

### `GITID_VERSION`

Pins a specific release tag. `gitid update` installs that tag instead of the latest release (equivalent to `gitid update --version <tag>`; the flag wins when both are given). Also read by the install scripts.

### `GH_TOKEN` / `GITHUB_TOKEN`

Token used to download releases when the repository is private and the `gh` CLI is not available or not logged in. `GH_TOKEN` is preferred; `GITHUB_TOKEN` is the fallback. Used by both `gitid update` and the install scripts.

### Installer-only: `GITID_INSTALL_DIR`

Read only by `scripts/install.sh` / `install.ps1`: where to put the binary (default `~/.local/bin`, Windows `%LOCALAPPDATA%\gitid\bin`). The installers also honour `GITID_REPO`, `GITID_VERSION`, and `GH_TOKEN`/`GITHUB_TOKEN` as above. See [Installation](../../getting-started/installation/).

## Variables gitid sets

These are exported by the shell hook (via `gitid env`) when you enter a directory mapped to a profile, and restored to their previous values when you leave.

### `GITID_PROFILE`

Always set to the active profile's name. Read-only signal for prompts (starship, powerlevel10k, …); nothing in gitid's git-side behaviour depends on it — the git identity comes from conditional includes, not the environment.

### `GH_CONFIG_DIR`

Set to `<data dir>/gh/<name>` when the active profile has gh isolation enabled (see [`[profiles.<name>.gh]`](../profiles-toml/)), so the GitHub CLI uses that profile's own auth. If you had your own `GH_CONFIG_DIR` before activation, it is saved and restored on deactivation.

### `GITID_STATE`

Internal: a base64-encoded JSON record of the active profile and the pre-activation values of every variable the hook set, so deactivation can restore them exactly. Treat it as opaque; do not set or modify it.

### Profile `[env]` variables

Any variables listed in a profile's `[profiles.<name>.env]` table are exported alongside the above and restored the same way.

## Other variables gitid consults

| Variable | Where |
|---|---|
| `NO_COLOR` | Any non-empty value disables coloured output (as does `--no-color` or a non-TTY stdout). |
| `PWD` | `gitid env` prefers the shell's logical `$PWD` over the physical cwd, so symlinked trees resolve the way your shell displays them. |
| `GIT_CONFIG_GLOBAL`, `XDG_CONFIG_HOME` | Used to resolve which global gitconfig file receives the `[include]` line, mirroring git's own precedence. |
| `XDG_CONFIG_HOME`, `XDG_DATA_HOME` | Standard XDG overrides for the default config/data locations on Linux and macOS. |
