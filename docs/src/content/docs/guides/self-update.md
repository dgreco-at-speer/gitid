---
title: Keeping gitid up to date
description: The gitid update command, the opportunistic daily background check, and the environment variables that control both.
---

gitid can update itself from GitHub releases and quietly tells you when a newer version exists. It never installs anything without you asking.

## `gitid update`

```sh
gitid update            # install the latest release
gitid update --check    # only report whether a newer version exists
gitid update --force    # reinstall even if already up to date
gitid update --version v0.3.0   # install a specific release tag
```

`gitid update` downloads the release archive for your platform and replaces the running binary **in place** — it swaps whichever `gitid` is currently executing, wherever it lives, so it works regardless of where the installer (or you) put it.

`--version <tag>` pins a specific release instead of the latest. When `--version` is omitted, the `GITID_VERSION` environment variable serves as the default pin. With a pinned version, gitid skips the install if you are already at exactly that version (unless `--force`).

### Authentication

Releases live in a private repository, so updating needs auth the same way the installer does. gitid prefers the [GitHub CLI](https://cli.github.com) (`gh auth login`) and falls back to `curl` with a `GH_TOKEN` (or `GITHUB_TOKEN`) bearer token. Either `gh` or `curl` must be installed.

### Platforms without prebuilt binaries

Prebuilt binaries do not exist for every platform — macOS in particular has none yet. On an unsupported platform, `gitid update` does not attempt a download; it points you at building from source instead:

```sh
cargo install --git https://github.com/dgreco-at-speer/gitid gitid
```

If you installed from source, this is also how you update.

## The opportunistic background check

gitid also checks for updates on its own, with strict limits:

- **Notify-only.** When a newer version is known, gitid prints a one-line notice — to **stderr**, never stdout, so pipelines and command substitution are unaffected. It never installs anything by itself.
- **Never blocks.** The notice comes from a cached result of a previous check (`~/.local/share/gitid/update-check.json`). When that cache is older than the check interval (default: 24 hours), gitid spawns a fully detached background worker to refresh it for next time; the foreground command never waits on the network.
- **Interactive sessions only.** The background refresh is only spawned when stderr is a terminal — never from scripts or CI.
- **Silent commands stay silent.** The check never runs after shell-eval'd and machine-facing commands: `gitid env`, `gitid hook`, `gitid completions`, and `gitid current` emit no extra bytes, and `gitid update` itself is excluded.

`gitid update --check` refreshes the same cache immediately (in the foreground, with a spinner) without installing.

## Environment variables

| Variable | Effect |
|---|---|
| `GITID_NO_UPDATE_CHECK` | Set to any non-empty value to disable the opportunistic check entirely (no notices, no background workers). |
| `GITID_UPDATE_INTERVAL` | Override the check throttle, in **seconds** (default `86400`, i.e. once a day). |
| `GITID_VERSION` | Pin a release tag; acts as the default for `gitid update --version`. |
| `GITID_REPO` | Update from a different `owner/repo` (default `dgreco-at-speer/gitid`; matches the install scripts). |
| `GH_TOKEN` / `GITHUB_TOKEN` | Token for the `curl` fallback when `gh` is unavailable (private repo access). |

For example, to opt out of the background check permanently:

```sh
# in your shell rc
export GITID_NO_UPDATE_CHECK=1
```

See [Environment variables](../../reference/environment-variables/) for the full list gitid reads and sets.
