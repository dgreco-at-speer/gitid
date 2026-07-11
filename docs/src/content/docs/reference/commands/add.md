---
title: gitid add
description: Create a new git identity profile, interactively or from flags.
sidebar:
  order: 1
  label: add
---

Create a new profile — a named git identity (user.name/user.email, optional SSH key, commit signing, and GitHub CLI isolation).

## Synopsis

```sh
gitid add [OPTIONS] [NAME]
```

## Options

| Flag | Description |
| --- | --- |
| `[NAME]` | Profile name (lowercase slug, matching `^[a-z0-9][a-z0-9_-]*$`). Prompted if omitted in interactive mode. |
| `--git-name <GIT_NAME>` | git `user.name`. Required with `--non-interactive`. |
| `--email <EMAIL>` | git `user.email`. Required with `--non-interactive`. |
| `--ssh-key <SSH_KEY>` | Path to the SSH private key (becomes `core.sshCommand` with `-o IdentitiesOnly=yes`). |
| `--ssh-agent-key <SELECTOR>` | Use a key held by the ssh-agent instead of a key file: a `SHA256:` fingerprint (prefix ok) or a key comment, as printed by `ssh-add -l`. Validated against the running agent. Conflicts with `--ssh-key`. |
| `--signing <SIGNING>` | Commit-signing method. Possible values: `ssh`, `openpgp`, `none`. |
| `--signing-key <SIGNING_KEY>` | Signing key: public-key path (`ssh`), key id (`openpgp`), or `agent` to sign with the profile's agent-held key. Required when `--signing` is `ssh` or `openpgp`. |
| `--sign-commits` | Sign commits by default (`commit.gpgsign = true`). |
| `--gh` | Provision an isolated `GH_CONFIG_DIR` for this profile. This is the default; the flag exists to override an earlier `--no-gh`. |
| `--no-gh` | Do not provision a gh config dir. |
| `--non-interactive` | Fail rather than prompt for missing fields. |

## Interactive wizard

Run `gitid add` with no flags (or only some) in a terminal and a wizard prompts for anything missing: git name and email, an SSH key picked from keys discovered under `~/.ssh` (or a path you type, or none), commit signing (`none`/`ssh`/`openpgp`, with the SSH public key path pre-filled from your chosen private key), and whether to isolate GitHub CLI auth. Any flags you did pass are used as answers and skipped.

When stdin is not a terminal, the wizard refuses and asks you to pass `--non-interactive` with `--git-name` and `--email`.

## Examples

Create a work profile entirely from flags, suitable for scripts:

```console
$ gitid add work --non-interactive --git-name "Jane Doe" --email jane@corp.example \
      --ssh-key ~/.ssh/id_work --signing ssh --signing-key ~/.ssh/id_work.pub --sign-commits
✓ added profile "work"
```

Start the wizard for a personal profile, answering prompts as you go:

```console
$ gitid add personal
> Git user.name: Jane
> Git user.email: jane@home.example
> SSH key for this identity: ~/.ssh/id_personal (ED25519)
> Commit signing: none
> Isolate GitHub CLI auth for this profile? Yes
✓ added profile "personal"
```

Use a key from the ssh-agent (a hardware token, Secretive, the Windows OpenSSH
agent, …) instead of a key file, and sign commits with it too:

```console
$ gitid add work --non-interactive --git-name "Jane Doe" --email jane@corp.example \
      --ssh-agent-key jane@corp.example --signing ssh --signing-key agent --sign-commits
✓ added profile "work"
```

Create a profile without GitHub CLI isolation:

```console
$ gitid add oss --non-interactive --git-name "Jane" --email jane@home.example --no-gh
✓ added profile "oss"
```

## Notes

- Adding a profile fails if a profile with that name already exists; use [`gitid edit`](../edit/) instead.
- `gitid add` runs a sync automatically after writing `profiles.toml`, regenerating the derived gitconfig fragments and bootstrapping the global include — no separate `gitid sync` needed.
- In non-interactive mode, gh isolation is enabled unless you pass `--no-gh`.

## See also

- [`gitid new`](../new/) — provision a profile from scratch, generating keys for you
- [`gitid use`](../use/) — assign the new profile to a directory tree
- [`gitid edit`](../edit/) — change a profile's fields later
- [profiles.toml reference](../../profiles-toml/) — the file this command writes
