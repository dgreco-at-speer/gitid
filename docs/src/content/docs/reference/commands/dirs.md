---
title: gitid dirs
description: List all directory→profile mappings.
sidebar:
  order: 8
  label: dirs
---

List every directory→profile mapping gitid knows about.

## Synopsis

```sh
gitid dirs [OPTIONS]
```

## Options

| Flag | Description |
| --- | --- |
| `--format <FORMAT>` | Output format. Possible values: `table` (default), `json`, `names`. |

## Examples

List the mappings as a table. The `ICASE` column shows `i` for mappings matched case-insensitively:

```console
$ gitid dirs
DIRECTORY               PROFILE   ICASE
/home/jane/code/work    work
/home/jane/code/oss     personal
```

Print just the mapped directories, one per line:

```console
$ gitid dirs --format names
/home/jane/code/work
/home/jane/code/oss
```

Dump the full mappings store as JSON, including any derived per-mapping environment (such as `GH_CONFIG_DIR`):

```console
$ gitid dirs --format json
{
  "version": 1,
  "mapping": [
    {
      "dir": "/home/jane/code/work/",
      "profile": "work",
      "case_insensitive": false,
      ...
    }
  ]
}
```

## Notes

- With no mappings yet, the table format prints a hint pointing at `gitid use <profile> [dir]`.
- Mappings are stored in `mappings.toml`, which is machine-owned — prefer [`gitid use`](../use/) and [`gitid forget`](../forget/) over editing it by hand.

## See also

- [`gitid use`](../use/) / [`gitid forget`](../forget/) — manage the mappings listed here
- [`gitid current`](../current/) — which mapping wins for a given directory
- [mappings.toml reference](../../files/)
