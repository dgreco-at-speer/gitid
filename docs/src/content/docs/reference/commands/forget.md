---
title: gitid forget
description: Remove the directory→profile mapping for a directory.
sidebar:
  order: 7
  label: forget
---

Remove the directory→profile mapping for a directory. The profile itself is untouched; the tree simply stops resolving a gitid identity.

## Synopsis

```sh
gitid forget [DIR]
```

## Options

| Flag | Description |
| --- | --- |
| `[DIR]` | Directory to unassign. Default: the current directory. |

## Examples

Forget the mapping for the current directory:

```console
$ cd ~/code/work
$ gitid forget
✓ forgot /home/jane/code/work
```

Forget a mapping from anywhere by naming the directory:

```console
$ gitid forget ~/code/oss
✓ forgot /home/jane/code/oss
```

If no mapping exists for the directory, the command fails:

```console
$ gitid forget ~/scratch
Error: no mapping for /home/jane/scratch
```

## Notes

- `forget` must be given the mapped directory itself, not a subdirectory of it — check [`gitid dirs`](../dirs/) for the exact path.
- The directory is looked up in canonical form first (matching how `gitid use` stored it), falling back to the literal path — so mappings can be forgotten even after the directory has been deleted.
- The mapping is removed from `mappings.toml` and a sync runs automatically, so the `includeIf` manifest is updated in the same step.

## See also

- [`gitid use`](../use/) — create a mapping
- [`gitid dirs`](../dirs/) — list all mappings
- [mappings.toml reference](../../files/)
