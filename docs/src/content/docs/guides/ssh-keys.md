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

## Keys held by the ssh-agent

A profile's key does not have to live on disk. Hardware tokens, Secretive, the
1Password agent, and the Windows OpenSSH agent all hold private keys that never
exist as files. Reference one by a *selector* instead of a path:

```toml
[profiles.work.ssh]
agent = "SHA256:eCEOtaIJH8nAEgkqjT98fzQPv4yYrLi3KNsphPVe1Lk"
# or by comment:  agent = "jane@corp.example"
```

The selector is either a `SHA256:` fingerprint (a unique prefix is enough) or a
key comment (exact match, falling back to a unique substring) — the same values
`ssh-add -l` prints. An ambiguous selector is an error that lists the matching
keys. On the command line, use `--ssh-agent-key`:

```sh
gitid add work --non-interactive \
  --git-name "Jane Doe" --email jane@corp.example \
  --ssh-agent-key jane@corp.example
```

The flag validates the selector against the running agent immediately, so typos
fail at `add` time rather than at the next sync. The wizard lists agent keys
alongside the ones discovered in `~/.ssh` (as `agent: …` entries) whenever an
agent is reachable.

### How it works

ssh needs a file to select the key by, even when the private half lives in the
agent. So `gitid sync` resolves the selector against the agent and writes the
key's **public** half to `~/.local/share/gitid/ssh/<name>.pub`. The generated
fragment points at that file:

```ini
[core]
	sshCommand = ssh -i /home/jane/.local/share/gitid/ssh/work.pub -o IdentitiesOnly=yes
```

When `-i` names a public key with no private key file next to it, ssh asks the
agent for the private operation — and `IdentitiesOnly=yes` still pins
authentication to exactly that key. Commit signing works through the same file:
set `key = "agent"` in the profile's `[signing]` table (see
[Commit signing](../commit-signing/)).

Because the profile stores only the selector, every sync re-resolves it. When
the agent is unreachable (or the key was removed from it), sync keeps the
previously materialised file and warns — repos keep working from the cached
public key. Sync only fails when the key was never materialised at all, since
the generated config would point at a file that doesn't exist.
[`gitid doctor`](../../reference/commands/doctor/) reports whether the agent is
reachable, whether each selector still resolves, and whether the materialised
copy is current.

### Finding the agent

gitid connects to the agent at `SSH_AUTH_SOCK`. On Windows, when the variable
is unset it falls back to the named pipe of the OpenSSH agent that ships with
Windows (`\\.\pipe\openssh-ssh-agent`).

:::caution
Git for Windows' bundled ssh cannot talk to the Windows agent's named pipe.
Put Windows' own OpenSSH (`C:\Windows\System32\OpenSSH`) first on `PATH` so
the plain `ssh` in the generated `core.sshCommand` resolves to it.
:::

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
mapping resolves, the generated files are current, and the key exists (on disk,
or in the agent for agent-held keys).
