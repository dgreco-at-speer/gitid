---
title: gitid doctor
description: Diagnose gitid configuration problems and get concrete fix hints.
sidebar:
  order: 10
  label: doctor
---

Runs a series of health checks over your gitid installation and reports each as OK (`✓`), a warning (`!`), or a failure (`✗`) with a concrete fix hint.

## Synopsis

```sh
gitid doctor [DIR]
```

## Options

| Flag | Description |
| --- | --- |
| `[DIR]` | Directory to inspect (default: current directory). Used for the git-resolution probe below. |

## What it checks

- **git version** — git is installed and at least the minimum supported version (2.13).
- **Global include** — your global gitconfig contains the `[include]` line pointing at gitid's generated manifest.
- **Generated files** — the per-profile fragments and the include manifest match what `gitid sync` would generate from `profiles.toml` and `mappings.toml` (stale files fail).
- **Mappings** — at least one directory mapping exists; every mapping points at a known profile (unknown profiles fail) and at a directory that still exists (missing directories warn).
- **Git resolution probe** — if `DIR` is inside a git repository and matches a mapping, doctor asks git what `user.email` actually resolves to there. If a repo-local or other config overrides the profile's email, it warns and names the overriding file.
- **SSH keys** — each profile's on-disk SSH key exists, and (on Unix) is not group/world readable.
- **ssh-agent keys** — when any profile uses an agent-held key: the agent is reachable, each selector resolves to exactly one key, and the materialised public key under the data dir is current (unreachable agents warn; a key that was never materialised fails).
- **Signing keys** — SSH-format signing keys exist on disk; a `"agent"` signing key requires the profile's SSH key to be agent-held. Warns when git is older than 2.34 (SSH signing needs it).
- **GitHub CLI** — if any profile enables gh isolation, the `gh` binary is available and each such profile has authenticated (`hosts.yml` present in its `GH_CONFIG_DIR`).
- **Shell hook** — a `gitid hook` line is present in `~/.bashrc`, `~/.zshrc`, or the fish config.

## Examples

Check the overall installation from anywhere:

```console
$ gitid doctor
✓ git 2.49
✓ global gitconfig includes gitid manifest
✓ generated files are up to date
✓ git resolves jane@corp.example in this repo
✓ work: gh auth configured
! personal: gh not authenticated
    → GH_CONFIG_DIR=/home/jane/.local/share/gitid/gh/personal gh auth login
✓ shell hook installed

all checks passed
```

Probe why a specific repository resolves the wrong identity:

```console
$ gitid doctor ~/code/work/api
✗ git resolves user.email = jane@home.example here, not jane@corp.example
    → overridden by .git/config; `git config --unset user.email` in that file
```

## Exit status

`0` when no check fails; `1` when at least one `✗` failure was reported. Warnings (`!`) do not affect the exit code.

## Notes

- The hook check only inspects the bash, zsh, and fish rc files, so PowerShell and nushell users may see a spurious "shell hook not detected" warning even after `gitid setup`.
- Most failures are fixed by running [`gitid sync`](../sync/).

## See also

- [`gitid sync`](../sync/) — regenerate the derived files doctor compares against.
- [`gitid current`](../current/) — quick per-directory identity check.
- [`gitid setup`](../setup/) — install the shell hook.
