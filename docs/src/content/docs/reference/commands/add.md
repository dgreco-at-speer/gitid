---
title: gitid add
description: Create a new git identity profile, interactively or from flags — optionally generating SSH/GPG keys.
sidebar:
  order: 1
  label: add
---

Create a new profile — a named git identity (user.name/user.email, optional SSH key, commit signing, and GitHub CLI isolation). `gitid add` can reference credentials you already have **or** generate new SSH/GPG keys from scratch.

## Synopsis

```sh
gitid add [OPTIONS] [NAME]
```

## Options

| Flag | Description |
| --- | --- |
| `[NAME]` | Profile name (lowercase slug, matching `^[a-z0-9][a-z0-9_-]*$`). Prompted if omitted in interactive mode. |
| `--git-name <GIT_NAME>` | git `user.name`. Required with `--non-interactive`. |
| `--email <EMAIL>` | git `user.email`. Required with `--non-interactive`. Also used as the generated SSH key's comment. |
| `--ssh-key <SSH_KEY>` | Path for the SSH private key. If the file exists it is reused; if not, a new keypair is generated there (becomes `core.sshCommand` with `-o IdentitiesOnly=yes`). |
| `--ssh-agent-key <SELECTOR>` | Use a key held by the ssh-agent instead of a key file: a `SHA256:` fingerprint (prefix ok) or a key comment, as printed by `ssh-add -l`. Validated against the running agent. Conflicts with `--ssh-key`. |
| `--ssh-type <SSH_TYPE>` | Algorithm for a generated key. Possible values: `ed25519` (default), `rsa`. |
| `--no-ssh` | Do not provision an SSH key for this profile. |
| `--signing <SIGNING>` | Commit-signing method. Possible values: `ssh`, `openpgp`, `none`. |
| `--signing-key <SIGNING_KEY>` | Signing key: public-key path (`ssh`), key id (`openpgp`), or `agent` to sign with the profile's agent-held key. When omitted, `ssh` reuses the profile's SSH key `.pub` and `openpgp` generates a GPG key. |
| `--sign-commits` | Sign commits by default (`commit.gpgsign = true`). |
| `--gh` | Provision an isolated `GH_CONFIG_DIR` for this profile. This is the default; the flag exists to override an earlier `--no-gh`. |
| `--no-gh` | Do not provision a gh config dir. |
| `--non-interactive` | Fail rather than prompt for missing fields, and take no interactive actions (no GitHub follow-up). |

## Interactive wizard

Run `gitid add` with no flags (or only some) in a terminal and a wizard prompts for anything missing:

1. **git name / email** — prompted if not passed as flags.
2. **SSH key** — pick **generate a new key** (the default), an existing key discovered under `~/.ssh`, a key held by the ssh-agent, a path you type, or none. Generating prompts for the storage path (default `~/.ssh/id_ed25519_<name>`) and then runs `ssh-keygen`, which prompts for a passphrase itself.
3. **Commit signing** — `none`, `ssh` (reuses the key you just generated), or `openpgp` (offers to generate a GPG key via `gpg --quick-generate-key`, or accepts an existing key id).
4. **GitHub CLI isolation** — whether to give this profile its own `GH_CONFIG_DIR`.

Any flags you did pass are used as answers and skipped. When stdin is not a terminal, the wizard refuses and asks you to pass `--non-interactive` with `--git-name` and `--email`.

After writing the profile and syncing, if gh isolation is on and the `gh` CLI is installed, it offers (default **no**) to run `gh auth login` and upload your new SSH/GPG public key — all scoped to the profile's isolated config dir. Whatever you decline, the exact commands are printed so you can run them later.

## Examples

Provision a work profile interactively, generating a key at the default path:

```console
$ gitid add work
> Git user.name: Jane Doe
> Git user.email: jane@corp.example
> SSH key for this identity: generate a new key
> Where to store the new SSH key: ~/.ssh/id_ed25519_work
✓ generated SSH key ~/.ssh/id_ed25519_work
> Commit signing: ssh
> Sign commits by default? Yes
> Isolate GitHub CLI auth for this profile? Yes
✓ added profile "work"
? Authenticate GitHub CLI for this profile now? No
› SSH public key: ~/.ssh/id_ed25519_work.pub
```

Create a work profile entirely from flags, referencing an existing key:

```console
$ gitid add work --non-interactive --git-name "Jane Doe" --email jane@corp.example \
      --ssh-key ~/.ssh/id_work --signing ssh --signing-key ~/.ssh/id_work.pub --sign-commits
✓ added profile "work"
```

Provision non-interactively (for scripts), generating the key at an explicit path:

```console
$ gitid add ci --non-interactive --git-name "CI Bot" --email ci@corp.example \
      --ssh-key ~/.ssh/id_ed25519_ci --signing ssh --sign-commits
✓ generated SSH key ~/.ssh/id_ed25519_ci
✓ added profile "ci"
```

Use a key from the ssh-agent (a hardware token, Secretive, the Windows OpenSSH agent, …) instead of a key file, and sign commits with it too:

```console
$ gitid add work --non-interactive --git-name "Jane Doe" --email jane@corp.example \
      --ssh-agent-key jane@corp.example --signing ssh --signing-key agent --sign-commits
✓ added profile "work"
```

Create a profile with no SSH key and no gh isolation:

```console
$ gitid add oss --non-interactive --git-name "Jane" --email jane@home.example --no-ssh --no-gh
✓ added profile "oss"
```

## Notes

- Adding a profile fails if a profile with that name already exists; use [`gitid edit`](../edit/) instead.
- SSH generation needs `ssh-keygen` on `PATH`; OpenPGP generation needs `gpg`; the GitHub follow-up needs `gh`. Each is checked, with a clear error or a printed manual command when missing.
- An existing private key at `--ssh-key` is reused rather than overwritten, so re-running with the same path is safe.
- `gitid add` runs a sync automatically after writing `profiles.toml`, regenerating the derived gitconfig fragments and bootstrapping the global include — no separate `gitid sync` needed.
- In non-interactive mode, gh isolation is enabled unless you pass `--no-gh`.

## See also

- [`gitid use`](../use/) — assign the new profile to a directory tree
- [`gitid edit`](../edit/) — change a profile's fields later
- [`gitid doctor`](../doctor/) — check whether keys and gh auth are in place
- [profiles.toml reference](../../profiles-toml/) — the file this command writes
