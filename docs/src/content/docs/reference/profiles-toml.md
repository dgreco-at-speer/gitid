---
title: profiles.toml
description: Complete annotated schema for profiles.toml, the hand-editable source of truth for gitid identities.
---

`profiles.toml` lives in the gitid config directory (`~/.config/gitid/profiles.toml` on Linux/macOS — see [Files](../files/)) and is the **source of truth** for your identities. It is designed to be edited by hand: gitid's own mutations go through a comment-preserving TOML editor, so your comments and key ordering survive `gitid add`/`gitid edit`.

Everything else gitid writes (gitconfig fragments, the include manifest, gh config dirs) is *derived* from this file. After editing it by hand, run [`gitid sync`](../commands/sync/) to regenerate the derived files.

:::note
Profile names (the `<name>` in `[profiles.<name>]`) become filenames and gitconfig keys, so they are restricted to a conservative slug: lowercase ASCII letters, digits, `-` and `_`, starting with a letter or digit (`^[a-z0-9][a-z0-9_-]*$`).
:::

## Full annotated example

Every field the schema supports:

```toml
# Schema version. Required. Must be 1 — gitid refuses to load any other value.
version = 1

[profiles.work]
name  = "Jane Doe"                  # required → user.name
email = "jane@corp.example"         # required → user.email

[profiles.work.ssh]                 # optional table
key = "~/.ssh/id_work"              # required within [ssh]; path to the PRIVATE key.
                                    # → core.sshCommand = "ssh -i <abs path> -o IdentitiesOnly=yes"

[profiles.work.signing]             # optional table
format  = "ssh"                     # required within [signing]: "ssh" | "openpgp" → gpg.format
key     = "~/.ssh/id_work.pub"      # required within [signing].
                                    #   ssh:     path to the PUBLIC key (expanded to absolute)
                                    #   openpgp: the key id, written verbatim
                                    # → user.signingkey
commits = true                      # optional, default false. true → commit.gpgsign = true
                                    # (false writes nothing; git's default applies)
tags    = true                      # optional. Only `true` writes tag.gpgsign = true;
                                    # false or absent writes nothing

[profiles.work.gh]                  # optional table. Presence of the table enables
enabled = true                      # gh isolation; `enabled` defaults to true, so a bare
                                    # [profiles.work.gh] is equivalent. Set false (or omit
                                    # the table) to disable. When enabled, the shell hook
                                    # exports an isolated GH_CONFIG_DIR for this profile.

[profiles.work.env]                 # optional: arbitrary env vars the shell hook exports
                                    # while this profile is active (restored on leave).
GLAB_CONFIG_DIR = "~/.config/glab-work"

[profiles.work.extra]               # optional: raw git config passthrough, written
                                    # verbatim into the generated fragment.
"core.autocrlf" = "input"
"url.git@github.com-work:.insteadOf" = "git@github.com:"
```

## Top level

| Field | Type | Required | Description |
|---|---|---|---|
| `version` | integer | yes | Schema version. Must be `1`; gitid errors on any other value. |
| `profiles.<name>` | table | — | One table per identity. The profile's name is the table key, not a field. |

## `[profiles.<name>]`

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Git author/committer name. |
| `email` | string | yes | Git author/committer email. |
| `ssh` | table | no | SSH key selection — see below. |
| `signing` | table | no | Commit/tag signing — see below. |
| `gh` | table | no | GitHub CLI isolation — see below. |
| `env` | table of strings | no | Extra env vars exported by the shell hook. |
| `extra` | table of strings | no | Raw git config passthrough. |

Generated git config:

```ini
[user]
	name = Jane Doe
	email = jane@corp.example
```

## `[profiles.<name>.ssh]`

| Field | Type | Required | Description |
|---|---|---|---|
| `key` | string | yes | Path to the SSH **private** key. A leading `~` is expanded to an absolute path in the generated config. |

Generated git config:

```ini
[core]
	sshCommand = ssh -i /home/jane/.ssh/id_work -o IdentitiesOnly=yes
```

`-o IdentitiesOnly=yes` ensures the right key is offered even when an ssh-agent holds several. gitid never exports `GIT_SSH_COMMAND` (an env var would override `core.sshCommand` for every repo in the shell).

## `[profiles.<name>.signing]`

| Field | Type | Required | Description |
|---|---|---|---|
| `format` | `"ssh"` \| `"openpgp"` | yes | Signing backend. Maps to `gpg.format`. |
| `key` | string | yes | For `ssh`: path to the **public** key, expanded to absolute. For `openpgp`: the key id, written verbatim. Maps to `user.signingkey`. |
| `commits` | boolean | no (default `false`) | `true` writes `commit.gpgsign = true`. `false` writes nothing (git's default applies). |
| `tags` | boolean | no | Only `true` writes `tag.gpgsign = true`; `false` or absent writes nothing. |

Generated git config:

```ini
[user]
	signingkey = /home/jane/.ssh/id_work.pub
[gpg]
	format = ssh
[commit]
	gpgsign = true
[tag]
	gpgsign = true
```

## `[profiles.<name>.gh]`

| Field | Type | Required | Description |
|---|---|---|---|
| `enabled` | boolean | no (default `true`) | Whether this profile gets an isolated GitHub CLI config dir. |

:::note
gh isolation is on only when the `[profiles.<name>.gh]` table is **present** and `enabled` is not `false`. Omitting the table entirely disables it — the `default true` applies to the field within the table, not to a missing table.
:::

This section produces no git config. Instead, `gitid sync` provisions a directory at `<data dir>/gh/<name>/` and the shell hook exports `GH_CONFIG_DIR` pointing at it while the profile is active, so each profile keeps its own `gh auth login` token. See [Environment variables](../environment-variables/).

## `[profiles.<name>.env]`

Free-form string-to-string table. Not git config — these variables are exported by the shell hook when the profile becomes active and restored to their previous values when it deactivates.

| Field | Type | Required | Description |
|---|---|---|---|
| `<VAR>` | string | — | Any variable name → value. Values containing a newline or NUL are rejected at sync time (they cannot be exported portably). |

## `[profiles.<name>.extra]`

Raw git config passthrough. Each key is a flattened git config key; each value is written verbatim (quoted per gitconfig rules when needed) into the generated fragment.

| Field | Type | Required | Description |
|---|---|---|---|
| `"<section>.<key>"` | string | — | Emitted as `[section]` `key = value`. |
| `"<section>.<subsection>.<key>"` | string | — | The section is everything up to the *last* dot; emitted as `[section "subsection"]` `key = value`. The subsection may itself contain dots. |

```toml
[profiles.work.extra]
"core.autocrlf" = "input"
"url.git@github.com-work:.insteadOf" = "git@github.com:"
```

generates:

```ini
[core]
	autocrlf = input
[url "git@github.com-work:"]
	insteadOf = git@github.com:
```

:::caution
`extra` values land in the profile's gitconfig fragment exactly as written, so they apply to **every repo** under directories mapped to the profile. Keys that collide with gitid-managed keys (`user.*`, `gpg.format`, `core.sshCommand`, …) will fight with the generated values.
:::

## After editing

Run [`gitid sync`](../commands/sync/) to regenerate the fragments, the include manifest, mapping env caches, and gh directories. The commands `add`, `edit`, `use`, `remove`, and `forget` run sync automatically; only hand edits need it explicitly.
