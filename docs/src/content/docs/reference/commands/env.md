---
title: gitid env
description: Print shell activation code for a directory — the shell hook's hot path.
sidebar:
  order: 12
  label: env
---

Prints the shell code that activates (or deactivates) the profile mapped to a directory. This is the hot path called by the shell hook on every directory change — you rarely need to run it yourself.

## Synopsis

```sh
gitid env --shell <SHELL> [--dir <DIR>]
```

## Options

| Flag | Description |
| --- | --- |
| `-s, --shell <SHELL>` | Required. Shell syntax to emit: `bash`, `zsh`, `fish`, `powershell` (alias `pwsh`), or `nu` (alias `nushell`). |
| `--dir <DIR>` | Resolve a specific directory instead of the current one. Without it, the logical `$PWD` is preferred over the physical cwd. |

## Behavior

- Looks up the directory in `mappings.toml`, diffs the target profile against the `GITID_STATE` carried in your environment, and prints only the changes — `export`/`unset` lines for `GITID_PROFILE`, `GH_CONFIG_DIR` (when gh isolation is enabled), any custom `[profiles.<name>.env]` variables, and the internal `GITID_STATE`.
- Prints **nothing** when nothing changed (the common case), so the hook stays cheap.
- Never breaks a prompt: any error goes to stderr and the command still exits `0` with empty stdout.
- If `GITID_DISABLE` is set to a non-empty value, it emits nothing at all (kill switch).

## Examples

See what the hook would do when entering a mapped directory (output is meant to be `eval`'d, not read):

```console
$ gitid env --shell bash --dir ~/code/work
export GITID_PROFILE='work'
export GH_CONFIG_DIR='/home/jane/.local/share/gitid/gh/work'
export GITID_STATE='eyJ2IjoxLCJwcm9maWxlIjoid29yayIsInNhdmVkIjp7Li4ufX0'
```

The way the hook actually consumes it:

```sh
eval "$(gitid env --shell bash)"
```

Run it again from the same state and it prints nothing:

```console
$ eval "$(gitid env --shell bash)"
$ gitid env --shell bash
$
```

## Notes

- Do not call `gitid env` manually for its side effects — it only *prints* code; nothing changes unless your shell evaluates it. To see which profile applies, use [`gitid current`](../current/) instead.
- Deactivation restores the previous values of every variable it set, using the saved diff in `GITID_STATE`. Do not set `GITID_STATE` yourself.

## See also

- [`gitid hook`](../hook/) — the per-shell script that calls `env` for you.
- [Environment variables](../../environment-variables/) — `GITID_PROFILE`, `GITID_STATE`, `GITID_DISABLE`.
- [Shell integration guide](../../../guides/shell-integration/)
