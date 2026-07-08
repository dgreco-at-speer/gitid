---
title: gitid current
description: Show which profile is active for a directory, cross-checked against git's actual resolution.
sidebar:
  order: 9
  label: current
---

Show which profile is active for a directory (the current directory by default), and — inside a repository — cross-check it against what git actually resolves.

## Synopsis

```sh
gitid current [OPTIONS] [DIR]
```

## Aliases

`gitid whoami`

## Options

| Flag | Description |
| --- | --- |
| `[DIR]` | Directory to inspect. Default: the current directory. |
| `--format <FORMAT>` | Output format. Possible values: `pretty` (default), `json`, `name`. |
| `-q`, `--quiet` | Print nothing; exit 0 if a profile is active, 1 otherwise. |

## Exit status

`0` when a profile is active for the directory, `1` when none is — with or without `--quiet`.

## Examples

Check the identity for the current directory:

```console
$ cd ~/code/work/api
$ gitid current
profile: work
  name:  Jane Doe
  email: jane@corp.example
```

Inside a git repository, `pretty` also compares the profile against git's real resolution and warns when a local override wins:

```console
$ gitid current ~/code/work/legacy-repo
profile: work
  name:  Jane Doe
  email: jane@corp.example
⚠ git resolves user.email = old@example.com here (from .git/config), not the profile's jane@corp.example; a local override may be set
```

Use `name` format in a prompt or script (prints nothing when no profile is active):

```console
$ gitid current --format name
work
```

Use `--quiet` as a condition:

```sh
if gitid current --quiet; then
  echo "identity managed by gitid"
fi
```

## Notes

- JSON output is `{"profile": "work", "email": "...", "dir": "..."}` when a mapping matches, and `{"profile": null}` when none does.
- The result comes from gitid's directory mappings; the cross-check against `git config user.email` (pretty format only, inside a repo) is what catches local overrides. For deeper diagnostics, run [`gitid doctor`](../doctor/).

## See also

- [`gitid dirs`](../dirs/) — all mappings
- [`gitid doctor`](../doctor/) — full diagnostics for a directory
- [`gitid list`](../list/) — the same active profile shown as a `●` marker
