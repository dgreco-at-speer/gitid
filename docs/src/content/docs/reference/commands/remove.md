---
title: gitid remove
description: Delete a profile, optionally dropping the directory mappings that use it.
sidebar:
  order: 5
  label: remove
---

Delete a profile from `profiles.toml` and regenerate the derived files.

## Synopsis

```sh
gitid remove [OPTIONS] <NAME>
```

## Aliases

`gitid rm`

## Options

| Flag | Description |
| --- | --- |
| `<NAME>` | The profile to remove (required). |
| `-f`, `--force` | Remove even if directories are mapped to it, and drop those mappings. |

## Examples

Remove an unused profile:

```console
$ gitid remove old-client
✓ removed profile "old-client"
```

If directories are still mapped to the profile, removal is refused and the mappings are listed:

```console
$ gitid remove work
✗ 1 director(ies) are mapped to "work":
    /home/jane/code/work/
Error: refusing to remove a profile in use; pass --force to remove it and its mappings
```

Force-remove the profile together with its mappings:

```console
$ gitid remove work --force
✓ removed profile "work"
ℹ left gh auth dir in place: /home/jane/.local/share/gitid/gh/work (delete manually if no longer needed)
```

## Notes

What gets deleted and what doesn't:

- **Deleted:** the profile entry in `profiles.toml`, its generated gitconfig fragment, and (with `--force`) any directory mappings pointing at it.
- **Never deleted:** the profile's gh config dir (`~/.local/share/gitid/gh/<name>/`). It holds GitHub CLI auth tokens, so gitid never removes it implicitly — a note reminds you to delete it manually if it exists.

A sync runs automatically after removal, so the global include and manifest are updated in the same step.

## See also

- [`gitid forget`](../forget/) — remove a single directory mapping without touching the profile
- [`gitid list`](../list/) — see what remains
