---
title: Directory mappings
description: Assign profiles to directory trees with gitid use, inspect mappings, and understand nested longest-path-wins resolution.
---

A **mapping** ties a directory tree to a profile. Every git repository under a
mapped tree resolves that profile's identity — with no per-repo configuration
and no need to `cd` anywhere. Mappings live in
`~/.config/gitid/mappings.toml` (machine-owned; prefer the commands below over
editing it).

If you haven't created a profile yet, start with
[Creating and managing profiles](../profiles/).

## Assigning: `gitid use`

```sh
gitid use <profile> [dir]
```

- `dir` defaults to the **current directory**, so `gitid use work` inside a
  repo onboards it (and everything beside/below it in that directory).
- It works **from anywhere** — the directory just has to exist. gitid
  canonicalizes the path (resolving symlinks) and stores it; no repo under it
  is touched.
- Running `gitid use` again on the same directory **replaces** the mapping —
  that's how you switch a tree to a different profile.

```sh
gitid use work ~/code/work          # from anywhere
gitid use personal .                # onboard the tree you're standing in
```

```console
$ gitid use work ~/code/work
✓ /home/jane/code/work now uses profile "work"
✓ synced: include regenerated
```

### Case sensitivity

By default matching is case-insensitive on Windows and macOS and
case-sensitive on Linux, matching each filesystem's convention. Override per
mapping with `--icase` (force insensitive) or `--no-icase` (force sensitive):

```sh
gitid use work /Volumes/CaseSensitive/work --no-icase
```

## Unassigning: `gitid forget`

```sh
gitid forget [dir]        # dir defaults to the current directory
```

```console
$ gitid forget ~/code/oss
✓ forgot /home/jane/code/oss
```

`forget` works even if the directory has since been deleted, and errors if
there is no mapping for it. It removes only the mapping — the profile itself
stays.

## Listing: `gitid dirs`

```sh
gitid dirs
```

```console
$ gitid dirs
DIRECTORY               PROFILE   ICASE
/home/jane/code         personal
/home/jane/code/work    work
```

Other formats:

```sh
gitid dirs --format names     # one directory per line
gitid dirs --format json      # full mapping objects, incl. derived env vars
```

## What's active here: `gitid current`

```sh
gitid current [dir]           # alias: gitid whoami
```

```console
$ gitid current ~/code/work/api
profile: work
  name:  Jane Doe
  email: jane@corp.example
```

When the directory is inside a git repository, `current` also cross-checks
what git *actually* resolves for `user.email` there and warns if a local
override in `.git/config` beats the profile.

Formats and scripting:

```sh
gitid current --format name        # just the profile name (nothing if none)
gitid current --format json        # {"profile":"work","email":"...","dir":"..."}
gitid current --quiet && echo mapped   # exit 0 if a profile is active, 1 if not
```

## Nested mappings: longest path wins

Mappings may nest. When several mapped directories contain the same path, the
**longest (most specific) directory wins** — both in gitid's own resolution
and in git's, because gitid writes the `includeIf` directives in ascending
path-length order so more specific fragments are applied last.

The standard recipe is a broad default plus specific subtrees:

```sh
gitid use personal ~/src            # everything under ~/src ...
gitid use work ~/src/work           # ... except ~/src/work
```

Resolution then looks like this:

```console
$ gitid current --format name ~/src/dotfiles
personal
$ gitid current --format name ~/src/work/api
work
$ gitid current --format name ~/src/work
work
$ gitid current --format name ~/videos
(no output — not under any mapping; exit code 1 with --quiet)
```

Add a third level and the same rule applies — `gitid use oss
~/src/work/upstream` carves an even more specific tree out of `work`.

:::tip
If a repo under a mapped tree still shows the wrong identity, run
`gitid doctor <dir>`. The usual culprits are a local `user.email` in that
repo's `.git/config` (which always beats includes) or a linked git worktree
whose *main* repository lives outside the mapped tree.
:::
