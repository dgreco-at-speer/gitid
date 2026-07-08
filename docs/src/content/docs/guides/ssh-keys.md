---
title: SSH keys per profile
description: Give each profile its own SSH key via core.sshCommand with IdentitiesOnly, and verify the right key is used.
---

Each profile can carry an SSH private key. Every repo under a directory mapped
to that profile then fetches and pushes with that key — no `~/.ssh/config`
host aliases, no per-repo remotes like `git@github.com-work:`.

## Setting a profile's key

Three equivalent ways:

**The wizard.** `gitid add` discovers candidate keys and presents a picker. It
scans `~/.ssh` by file *content* (any private key is found regardless of its
name) and also offers paths referenced by `IdentityFile` directives in
`~/.ssh/config`, showing each key's algorithm and comment:

```console
? SSH key for this profile:
> ~/.ssh/id_work  (ed25519, jane@corp.example)
  ~/.ssh/id_personal  (ed25519, jane@home.example)
  enter a path manually
  none
```

**Flags.** On `gitid add` or `gitid edit`:

```sh
gitid add work --non-interactive \
  --git-name "Jane Doe" --email jane@corp.example \
  --ssh-key ~/.ssh/id_work

gitid edit work --ssh-key ~/.ssh/id_work_new
```

**profiles.toml.** Then run `gitid sync`:

```toml
[profiles.work.ssh]
key = "~/.ssh/id_work"
```

The key is the **private** key path. `~` is fine — gitid expands it when
generating config. See the
[profiles.toml reference](../../reference/profiles-toml/).

## What gitid generates

The profile's generated gitconfig fragment
(`~/.local/share/gitid/profiles/work.gitconfig`) gets a `core.sshCommand`
with the key path expanded to absolute:

```ini
[core]
	sshCommand = ssh -i /home/jane/.ssh/id_work -o IdentitiesOnly=yes
```

Git runs this command instead of plain `ssh` for every fetch/push in repos the
profile applies to.

### Why `IdentitiesOnly=yes` matters

Without it, `-i` merely *adds* a key to the candidates: ssh still tries every
key in your agent first, in agent order. If your agent holds both a work and a
personal key, GitHub authenticates you as whichever key it sees first — and
GitHub maps keys to accounts, so the wrong key means the wrong account (or a
`Permission denied` / wrong-repo-access error). `IdentitiesOnly=yes` restricts
authentication to exactly the configured key.

:::note
gitid does **not** switch `SSH_AUTH_SOCK` or manage your agent. Your agent can
keep holding all your keys; `IdentitiesOnly` makes git ignore the extras. (For
encrypted keys the agent is still consulted for the *configured* key, so you
won't retype passphrases.)
:::

### Why gitid never exports `GIT_SSH_COMMAND`

The environment variable `GIT_SSH_COMMAND` overrides `core.sshCommand`
**globally** — for every repo in that shell, whatever tree it's in. Exporting
it from the shell hook would make one profile's key leak into every other
profile's repos, defeating the per-directory design. So gitid intentionally
carries SSH config only through git config, which resolves per repository.
Don't export it yourself either.

## Worked example: two GitHub accounts

One machine, a work GitHub account and a personal one, each with its own key:

```sh
gitid add work --non-interactive \
  --git-name "Jane Doe" --email jane@corp.example --ssh-key ~/.ssh/id_work
gitid add personal --non-interactive \
  --git-name "Jane" --email jane@home.example --ssh-key ~/.ssh/id_personal

gitid use personal ~/src
gitid use work ~/src/work
```

Now cloning with the standard remote URL just works in both trees:

```sh
git -C ~/src clone git@github.com:jane/dotfiles.git         # personal key
git -C ~/src/work clone git@github.com:corp/api.git         # work key
```

No `github.com-work` host aliases needed — the same `git@github.com:` URL
authenticates as the right account depending on where the repo lives.

## Testing which key git will use

Check what git resolves in a given repo:

```sh
git -C ~/src/work/api config core.sshCommand
```

```console
ssh -i /home/jane/.ssh/id_work -o IdentitiesOnly=yes
```

GitHub tells you which account a key maps to. Test a key directly the same way
gitid's generated command would use it:

```sh
ssh -i ~/.ssh/id_work -o IdentitiesOnly=yes -T git@github.com
```

```console
Hi jane-corp! You've successfully authenticated, but GitHub does not provide shell access.
```

For a one-off test *inside* a repo you can use `GIT_SSH_COMMAND` as a manual
override (this is exactly why gitid never exports it — it beats
`core.sshCommand`):

```sh
GIT_SSH_COMMAND="ssh -v" git -C ~/src/work/api fetch   # -v shows the key offered
```

If the wrong key wins, run `gitid doctor ~/src/work/api` — it checks that the
mapping resolves, the generated files are current, and the key file exists.
