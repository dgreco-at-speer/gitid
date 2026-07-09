---
title: Per-profile GitHub CLI auth
description: Give each gitid profile its own isolated gh login by exporting a per-profile GH_CONFIG_DIR through the shell hook.
---

The GitHub CLI (`gh`) normally stores one login per machine, in `~/.config/gh`. gitid can give each profile its own isolated `gh` auth instead, so `gh pr create` in a work directory uses your work account and the same command in a personal directory uses your personal account — with no `gh auth switch` juggling.

:::caution[Requires the shell hook]
This feature runs entirely through the shell hook. Identity (name, email, SSH key, signing) works without it, but `gh` isolation does not — the hook is what exports `GH_CONFIG_DIR` when you `cd`. Install it with `gitid setup`. See [Shell integration](../shell-integration/).
:::

## Enabling gh isolation

gh isolation is **enabled by default** when you create a profile. `gitid add` provisions the isolated config directory unless you opt out:

```sh
gitid add work --git-name "Jane Doe" --email jane@corp.example   # gh isolation on
gitid add scratch --git-name "Jane" --email jane@home.example --no-gh   # opted out
```

`--gh` exists to override an earlier `--no-gh` (for example when a wrapper script always passes `--no-gh`). In the interactive wizard, a "Isolate GitHub CLI auth for this profile?" prompt covers the same choice.

`gitid edit` has no gh flag. To toggle it on an existing profile, edit `~/.config/gitid/profiles.toml` and run `gitid sync`:

```toml
[profiles.work.gh]
enabled = true
```

```sh
gitid sync
```

## How it works

For every profile with gh enabled, `gitid sync` provisions a directory under the data dir:

```
~/.local/share/gitid/gh/<profile>/    # this profile's GH_CONFIG_DIR (gh tokens live here)
```

When you `cd` into a mapped directory, the shell hook resolves the directory→profile mapping and exports `GH_CONFIG_DIR` pointing at that profile's directory. `gh` reads all of its config and auth from `GH_CONFIG_DIR`, so each profile gets a fully separate login. When you leave the mapped tree, the hook restores whatever value (or absence) `GH_CONFIG_DIR` had before.

## First-time setup

Each profile's gh config starts empty, so you log in once per profile. `cd` into a directory mapped to the profile — the hook exports the isolated `GH_CONFIG_DIR` — and log in as usual:

```sh
cd ~/code/work
gh auth login        # lands in ~/.local/share/gitid/gh/work/, not your global gh config
```

Verify which account is active:

```sh
gh auth status
```

You can also confirm the hook did its job:

```console
$ echo $GH_CONFIG_DIR
/home/jane/.local/share/gitid/gh/work
```

`gitid doctor` checks this end to end: it warns if `gh` is not installed while profiles enable isolation, and reports per profile whether `gh` is authenticated yet (with the exact `GH_CONFIG_DIR=… gh auth login` command to fix it).

## Worked example: separate work and personal GitHub accounts

```sh
# Two profiles; gh isolation is on by default for both
gitid add work --git-name "Jane Doe" --email jane@corp.example --ssh-key ~/.ssh/id_work
gitid add personal --git-name "Jane" --email jane@home.example --ssh-key ~/.ssh/id_personal

# Map the trees (from anywhere)
gitid use work ~/code/work
gitid use personal ~/code/oss

# Log in once per profile, inside each tree
cd ~/code/work && gh auth login      # authenticate as jane-corp
cd ~/code/oss  && gh auth login      # authenticate as jane

# From now on, gh follows the directory
cd ~/code/work/api && gh auth status   # → jane-corp
cd ~/code/oss/tool && gh auth status   # → jane
```

Outside any mapped tree, `GH_CONFIG_DIR` is restored to its prior value, so your global `gh` login (if any) is untouched.

## Removal does not delete tokens

`gitid remove <profile>` deletes the profile but **never** deletes its gh auth directory — it holds live tokens, so gitid leaves it in place and prints where it is:

```console
$ gitid remove work --force
✓ removed profile "work"
left gh auth dir in place: /home/jane/.local/share/gitid/gh/work (delete manually if no longer needed)
```

Delete it yourself once you are sure you no longer need the login:

```sh
rm -r ~/.local/share/gitid/gh/work
```

## Troubleshooting

If `gh` uses the wrong account inside a mapped directory, the hook is almost always the culprit — see [Common issues](../../troubleshooting/common-issues/). Quick check: `echo $GH_CONFIG_DIR` in the mapped directory; if it is empty, the hook is not loaded in this shell (`gitid setup`, then restart the shell).
