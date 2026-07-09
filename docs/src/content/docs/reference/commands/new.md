---
title: gitid new
description: Provision a new git identity from scratch — generate SSH/GPG keys and set up GitHub CLI.
sidebar:
  order: 1.5
  label: new
---

Provision a profile from scratch. Where [`gitid add`](../add/) references credentials you already have, `gitid new` **creates** them: it generates an SSH keypair (asking where to store it, with a sensible default), optionally generates a GPG signing key, provisions the isolated `GH_CONFIG_DIR`, and offers to authenticate the GitHub CLI and upload your new public key.

## Synopsis

```sh
gitid new [OPTIONS] [NAME]
```

## Options

| Flag | Description |
| --- | --- |
| `[NAME]` | Profile name (lowercase slug, matching `^[a-z0-9][a-z0-9_-]*$`). Prompted if omitted in interactive mode. |
| `--git-name <GIT_NAME>` | git `user.name`. Required with `--non-interactive`. |
| `--email <EMAIL>` | git `user.email`. Required with `--non-interactive`. Also used as the generated SSH key's comment. |
| `--ssh-key <SSH_KEY>` | Where to write the new SSH private key. Defaults to `~/.ssh/id_ed25519_<name>` in interactive mode. An existing file at this path is **reused, not overwritten**. |
| `--ssh-type <SSH_TYPE>` | Algorithm for the generated key. Possible values: `ed25519` (default), `rsa`. |
| `--no-ssh` | Do not provision an SSH key for this profile. |
| `--signing <SIGNING>` | Commit-signing method. Possible values: `ssh`, `openpgp`, `none`. |
| `--signing-key <SIGNING_KEY>` | Use an existing signing key instead of generating one: public-key path (`ssh`) or key id (`openpgp`). When omitted, `ssh` reuses the generated key's `.pub` and `openpgp` generates a new GPG key. |
| `--sign-commits` | Sign commits by default (`commit.gpgsign = true`). |
| `--gh` | Provision an isolated `GH_CONFIG_DIR` for this profile. This is the default; the flag exists to override an earlier `--no-gh`. |
| `--no-gh` | Do not provision a gh config dir. |
| `--non-interactive` | Fail rather than prompt for missing fields, and take no interactive actions (no GitHub follow-up). |

## Interactive wizard

Run `gitid new <name>` in a terminal and it walks you through the whole setup:

1. **git name / email** — prompted if not passed as flags.
2. **SSH key** — pick **generate a new key** (the default), an existing key discovered under `~/.ssh`, a path you type, or none. Generating prompts for the storage path (default `~/.ssh/id_ed25519_<name>`) and then runs `ssh-keygen`, which prompts for a passphrase itself.
3. **Commit signing** — `ssh` (reuses the key you just generated), `openpgp` (offers to generate a GPG key via `gpg --quick-generate-key`, or accepts an existing key id), or `none`.
4. **GitHub CLI isolation** — whether to give this profile its own `GH_CONFIG_DIR`.

After writing the profile and syncing, if gh isolation is on and the `gh` CLI is installed, it offers (default **no**) to run `gh auth login` and upload your new SSH/GPG public key — all scoped to the profile's isolated config dir. Whatever you decline, the exact commands are printed so you can run them later.

When stdin is not a terminal, the wizard refuses and asks you to pass `--non-interactive` with `--git-name` and `--email`.

## Examples

Provision a work profile interactively, generating a key at the default path:

```console
$ gitid new work
> Git user.name: Jane Doe
> Git user.email: jane@corp.example
> SSH key for this identity: generate a new key
> Where to store the new SSH key: ~/.ssh/id_ed25519_work
✓ generated SSH key ~/.ssh/id_ed25519_work
> Commit signing: ssh
> Sign commits by default? Yes
> Isolate GitHub CLI auth for this profile? Yes
✓ created profile "work"
? Authenticate GitHub CLI for this profile now? No
› SSH public key: ~/.ssh/id_ed25519_work.pub
```

Provision non-interactively (for scripts), generating the key at an explicit path:

```console
$ gitid new ci --non-interactive --git-name "CI Bot" --email ci@corp.example \
      --ssh-key ~/.ssh/id_ed25519_ci --signing ssh --sign-commits
✓ generated SSH key ~/.ssh/id_ed25519_ci
✓ created profile "ci"
```

Provision a profile with no SSH key and no gh isolation:

```console
$ gitid new oss --non-interactive --git-name "Jane" --email jane@home.example --no-ssh --no-gh
✓ created profile "oss"
```

## Notes

- `gitid new` fails if a profile with that name already exists; use [`gitid edit`](../edit/) instead.
- SSH generation needs `ssh-keygen` on `PATH`; OpenPGP generation needs `gpg`; the GitHub follow-up needs `gh`. Each is checked, with a clear error or a printed manual command when missing.
- An existing private key at `--ssh-key` is reused rather than overwritten, so re-running with the same path is safe.
- Like `gitid add`, this runs a sync automatically after writing `profiles.toml` — no separate `gitid sync` needed.

## See also

- [`gitid add`](../add/) — create a profile from credentials you already have
- [`gitid use`](../use/) — assign the new profile to a directory tree
- [`gitid doctor`](../doctor/) — check whether keys and gh auth are in place
- [profiles.toml reference](../../profiles-toml/) — the file this command writes
