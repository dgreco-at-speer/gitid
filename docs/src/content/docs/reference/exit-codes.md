---
title: Exit codes
description: Process exit codes for gitid commands, including the query semantics of current, doctor, and env.
---

gitid uses only three exit codes:

| Code | Meaning |
|---|---|
| `0` | Success — or the "true" answer for query commands. |
| `1` | Error (message on stderr) — or the "false" answer for query commands. |
| `2` | Command-line usage error (unknown flag or subcommand); the argument parser prints usage to stderr. |

There are no command-specific codes beyond these; any runtime failure (unreadable store, unparsable TOML, failed git subprocess, failed download) exits `1` with a human-readable message on stderr.

## Query commands

Some commands use the exit code as an answer, not just an error signal:

| Command | `0` | `1` |
|---|---|---|
| `gitid current [dir]` | a profile is active for the directory | no profile active (also without `--quiet`) |
| `gitid current --quiet [dir]` | profile active (prints nothing) | no profile active (prints nothing) |
| `gitid doctor [dir]` | all checks passed | problems found (marked `✗` in the output) |

```sh
if gitid current --quiet; then
  echo "identity managed here"
fi
```

:::note
Plain `gitid current` (without `--quiet`) also exits `1` when no profile is active for the directory, even though it prints a friendly "no profile active" message (or `{"profile":null}` with `--format json`) rather than an error.
:::

## `gitid env` always exits 0

`gitid env` is the hot path evaluated by the shell hook on every directory change. It must never break a prompt, so it exits `0` even when something goes wrong: errors are printed to stderr and stdout stays empty. Setting `GITID_DISABLE` also results in empty output with exit `0`. See [Environment variables](../environment-variables/).

## Everything else

`add`, `edit`, `use`, `forget`, `remove`, `list`, `dirs`, `show`, `sync`, `init`, `setup`, `hook`, `completions`, and `update` follow the plain convention: `0` on success, `1` on any error. Notably:

- `gitid remove <name>` exits `1` if directories still map to the profile and `--force` was not given.
- `gitid update --check` exits `0` whether or not a newer version exists — it reports via output, not the exit code.
