---
title: Commit signing
description: Configure SSH or OpenPGP commit signing per profile, see the git config gitid generates, and verify signed commits.
---

A profile can carry a commit-signing setup, so repos under a work tree sign
with your work key and repos elsewhere sign with another key — or not at all.

## The two formats

git supports two signing backends, selected by `gpg.format`:

- **`ssh`** — sign with an SSH key. The signing `key` is the path to the
  **public** key (e.g. `~/.ssh/id_work.pub`). No GPG keyring needed; GitHub
  and GitLab verify these when you upload the public key as a *signing key*.
- **`openpgp`** — classic GPG. The signing `key` is a **key id** (or
  fingerprint/email that `gpg` can resolve), not a file path.

Per profile you also choose whether commits and/or tags are signed by default
(`commit.gpgsign` / `tag.gpgsign`).

## What gitid generates

From the profile's `[signing]` table, the generated fragment
(`~/.local/share/gitid/profiles/<name>.gitconfig`) gets:

| profiles.toml field | generated git config |
|---|---|
| `format` | `gpg.format` (`ssh` or `openpgp`) |
| `key` | `user.signingkey` (ssh: path expanded to absolute; openpgp: written verbatim) |
| `commits = true` | `commit.gpgsign = true` |
| `tags = true` | `tag.gpgsign = true` |

For example:

```ini
[user]
	name = Jane Doe
	email = jane@corp.example
	signingkey = /home/jane/.ssh/id_work.pub
[gpg]
	format = ssh
[commit]
	gpgsign = true
```

`commit.gpgsign`/`tag.gpgsign` lines are only emitted when true; a profile
with `commits = false` can still sign on demand with `git commit -S`.

## Worked example: SSH signing

The flags form — `--signing ssh` requires `--signing-key`, and
`--sign-commits` turns on signing by default:

```sh
gitid add work --non-interactive \
  --git-name "Jane Doe" --email jane@corp.example \
  --ssh-key ~/.ssh/id_work \
  --signing ssh --signing-key ~/.ssh/id_work.pub --sign-commits
```

The equivalent `profiles.toml` (edit by hand, then `gitid sync`):

```toml
[profiles.work]
name  = "Jane Doe"
email = "jane@corp.example"

[profiles.work.ssh]
key = "~/.ssh/id_work"

[profiles.work.signing]
format  = "ssh"
key     = "~/.ssh/id_work.pub"
commits = true
tags    = true
```

In the interactive wizard (`gitid add` with no flags), if you picked an SSH
key the wizard offers its `.pub` sibling as the default signing key — reusing
your auth key for signing is the common setup.

:::note
`tags` has no CLI flag — set it in `profiles.toml`. Likewise, `gitid edit` has
no `--sign-commits` flag; to toggle default signing on an existing profile,
edit `commits` in `profiles.toml` and run `gitid sync`.
:::

## Worked example: OpenPGP signing

Find your key id, then use it (not a path) as the signing key:

```sh
gpg --list-secret-keys --keyid-format long
gitid add oss --non-interactive \
  --git-name "Jane Doe" --email jane@home.example \
  --signing openpgp --signing-key 3AA5C34371567BD2 --sign-commits
```

```toml
[profiles.oss.signing]
format  = "openpgp"
key     = "3AA5C34371567BD2"
commits = true
```

## Changing signing on an existing profile

```sh
gitid edit work --signing openpgp --signing-key ABCD1234   # switch format
gitid edit work --signing-key ~/.ssh/id_new.pub --signing ssh
gitid edit work --signing none                             # remove signing
```

When switching formats, the existing key is kept unless you pass
`--signing-key`; setting signing for the first time requires it. The
commits/tags booleans are preserved across edits.

## Verifying a signed commit

Make a commit under the mapped tree and inspect it:

```sh
cd ~/code/work/api
git commit --allow-empty -m "signing test"
git log --show-signature -1
```

```console
commit 4f2a…
Good "git" signature for jane@corp.example with ED25519 key SHA256:…
Author: Jane Doe <jane@corp.example>
```

`git verify-commit HEAD` does the same check with a pass/fail exit code.

### SSH signing: allowed signers

For **local** verification of SSH signatures, git needs an allowed-signers
file mapping emails to public keys — otherwise `--show-signature` reports
`No principal matched` even for your own commits (GitHub/GitLab verify
server-side regardless, using the signing key you upload to your account).

```sh
echo "jane@corp.example $(cat ~/.ssh/id_work.pub)" >> ~/.config/git/allowed_signers
```

gitid doesn't manage this file, but you can point git at it per profile via
the raw config passthrough in `profiles.toml`:

```toml
[profiles.work.extra]
"gpg.ssh.allowedSignersFile" = "/home/jane/.config/git/allowed_signers"
```

See the [profiles.toml reference](../../reference/profiles-toml/) for the
`[signing]` and `[extra]` schemas.
