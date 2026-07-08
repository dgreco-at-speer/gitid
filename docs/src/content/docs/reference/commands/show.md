---
title: gitid show
description: Show a single profile's full details, including its generated fragment and mapped directories.
sidebar:
  order: 3
  label: show
---

Show one profile's details: identity fields, SSH key, signing setup, gh isolation, the generated gitconfig fragment path, and every directory mapped to it.

## Synopsis

```sh
gitid show [OPTIONS] <NAME>
```

## Options

| Flag | Description |
| --- | --- |
| `<NAME>` | The profile to show (required). |
| `--format <FORMAT>` | Output format. Possible values: `pretty` (default), `json`. |

## Examples

Show a profile in human-readable form:

```console
$ gitid show work
profile: work
  name:   Jane Doe
  email:  jane@corp.example
  ssh:    ~/.ssh/id_work
  signing: ssh key=~/.ssh/id_work.pub commits=true
  gh:     enabled
  fragment: /home/jane/.local/share/gitid/profiles/work.gitconfig
  directories:
    /home/jane/code/work/
```

Emit the profile as JSON (the raw profile record, without fragment path or directories):

```console
$ gitid show work --format json
{
  "name": "Jane Doe",
  "email": "jane@corp.example",
  "ssh": { "key": "~/.ssh/id_work" },
  ...
}
```

## Notes

- The pretty format includes the path of the generated per-profile gitconfig fragment and the directories currently mapped to the profile; the JSON format contains only the profile record itself.
- An unknown name fails with a suggestion of close matches among existing profiles.

## See also

- [`gitid list`](../list/) — all profiles at a glance
- [`gitid edit`](../edit/) — change the fields shown here
- [profiles.toml reference](../../profiles-toml/)
