---
title: gitid use
description: Assign a profile to a directory tree so every repo under it picks up that identity.
sidebar:
  order: 6
  label: use
---

Assign a profile to a directory tree. Every git repository under that directory then resolves the profile's identity — no `cd` required and no repo config touched.

## Synopsis

```sh
gitid use [OPTIONS] <PROFILE> [DIR]
```

## Options

| Flag | Description |
| --- | --- |
| `<PROFILE>` | The profile to assign (required; must already exist). |
| `[DIR]` | Directory to assign. Default: the current directory. |
| `--icase` | Force case-insensitive directory matching. |
| `--no-icase` | Force case-sensitive directory matching. |

When neither `--icase` nor `--no-icase` is given, the platform default applies: case-insensitive on Windows and macOS, case-sensitive on Linux.

## Examples

Assign a profile to a tree from anywhere — the directory must exist, but you don't need to be in it:

```console
$ gitid use work ~/code/work
✓ /home/jane/code/work now uses profile "work"
```

Assign the current directory:

```console
$ cd ~/code/oss/some-project
$ gitid use personal
✓ /home/jane/code/oss/some-project now uses profile "personal"
```

Map a tree on a case-insensitive filesystem mount explicitly:

```console
$ gitid use work /mnt/shared/Projects --icase
✓ /mnt/shared/Projects now uses profile "work"
```

## Notes

- The directory is canonicalized (symlinks resolved) before being stored, and must exist. If the literal path differs from the canonical one (e.g. it goes through a symlink), both forms are recorded so git matches either.
- Running `use` again for the same directory replaces the existing mapping — that's how you switch a tree to a different profile.
- The mapping is written to `mappings.toml` and a sync runs automatically, regenerating the `includeIf` manifest so git picks up the identity immediately.
- Mappings apply to the whole tree: the deepest matching directory wins when trees nest.

## See also

- [`gitid forget`](../forget/) — remove a mapping
- [`gitid dirs`](../dirs/) — list all mappings
- [`gitid current`](../current/) — check what resolves for a directory
- [mappings.toml reference](../../files/)
