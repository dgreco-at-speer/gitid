---
title: gitid edit
description: Change an existing profile's fields from the command line.
sidebar:
  order: 4
  label: edit
---

Change an existing profile's fields. Only the flags you pass are modified; everything else is preserved.

## Synopsis

```sh
gitid edit [OPTIONS] <NAME>
```

## Options

| Flag | Description |
| --- | --- |
| `<NAME>` | The profile to edit (required). |
| `--git-name <GIT_NAME>` | Set git `user.name`. |
| `--email <EMAIL>` | Set git `user.email`. |
| `--ssh-key <SSH_KEY>` | Set the path to the SSH private key. |
| `--signing <SIGNING>` | Set the commit-signing method. Possible values: `ssh`, `openpgp`, `none` (removes signing). |
| `--signing-key <SIGNING_KEY>` | Signing key: public-key path (`ssh`) or key id (`openpgp`). Required with `--signing ssh`/`openpgp` unless the profile already has one. |
| `--open` | Open `profiles.toml` in `$EDITOR`, then sync. **Not implemented yet** — the command currently errors with the path to edit by hand. |

## Examples

Update a profile's email address:

```console
$ gitid edit work --email jane.doe@corp.example
✓ updated profile "work"
```

Enable SSH commit signing on an existing profile:

```console
$ gitid edit work --signing ssh --signing-key ~/.ssh/id_work.pub
✓ updated profile "work"
```

Turn signing off entirely:

```console
$ gitid edit work --signing none
✓ updated profile "work"
```

## Notes

- `gitid edit` syncs automatically after saving — the derived gitconfig fragments are regenerated for you, so no separate `gitid sync` is needed.
- `--open` is declared but not implemented yet; until it lands, edit `~/.config/gitid/profiles.toml` directly and run [`gitid sync`](../sync/) afterwards.
- When switching `--signing` between `ssh` and `openpgp`, the existing key and sign-commits setting are kept unless you override the key with `--signing-key`. There is no flag to toggle sign-commits here; set `commits = true`/`false` in `profiles.toml` and sync.
- Fields not editable by flag (gh isolation, `env`, `extra`, `tags` signing) are edited in `profiles.toml` by hand, followed by `gitid sync`.

## See also

- [`gitid show`](../show/) — inspect the result
- [`gitid sync`](../sync/) — regenerate after hand-edits
- [profiles.toml reference](../../profiles-toml/)
