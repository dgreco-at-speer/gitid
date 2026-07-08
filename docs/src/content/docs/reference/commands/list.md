---
title: gitid list
description: List all profiles, marking the one active for the current directory.
sidebar:
  order: 2
  label: list
---

List all profiles, with a `●` marker next to the profile active for the current directory.

## Synopsis

```sh
gitid list [OPTIONS]
```

## Aliases

`gitid ls`

## Options

| Flag | Description |
| --- | --- |
| `--format <FORMAT>` | Output format. Possible values: `table` (default), `json`, `names`. |

## Examples

List profiles as a table. The `●` in the first column marks the profile whose directory mapping covers your current working directory:

```console
$ gitid list
    NAME      GIT NAME  EMAIL              GH
  ● work      Jane Doe  jane@corp.example  yes
    personal  Jane      jane@home.example  yes
```

Print just the profile names, one per line — handy for shell loops and completion:

```console
$ gitid list --format names
work
personal
```

Dump the full profile store as JSON for scripting:

```console
$ gitid list --format json
{
  "version": 1,
  "profiles": {
    "work": {
      "name": "Jane Doe",
      "email": "jane@corp.example",
      ...
    }
  }
}
```

## Notes

- The active marker is derived from the directory→profile mappings, not from git — use [`gitid current`](../current/) to cross-check what git actually resolves.
- With no profiles yet, the table format prints a hint pointing at `gitid add`.

## See also

- [`gitid show`](../show/) — full details for one profile
- [`gitid dirs`](../dirs/) — the directory mappings behind the `●` marker
