---
title: Prompt integration
description: Show the active gitid profile in your shell prompt using the GITID_PROFILE environment variable.
---

The shell hook exports `GITID_PROFILE` — the name of the profile active for your current directory. Inside a mapped tree it holds the profile name; outside any mapped tree it is unset (or restored to whatever value it had before). That makes it trivial to surface the active identity in your prompt.

:::note
This requires the shell hook: `GITID_PROFILE` is set by the hook on directory change, not by git. Install it with `gitid setup` if you have not already. See [Shell integration](../shell-integration/).
:::

## Quick check

```console
$ cd ~/code/work && echo $GITID_PROFILE
work
$ cd ~ && echo $GITID_PROFILE

```

## starship

Add an `env_var` module to `~/.config/starship.toml`:

```toml
[env_var.GITID_PROFILE]
format = "[$env_value]($style) "
style = "bold yellow"
```

starship renders nothing when the variable is unset, so the segment only appears inside mapped trees.

## powerlevel10k

Define a custom segment in `~/.zshrc` (or wherever your p10k config lives):

```zsh
function prompt_gitid() { [[ -n $GITID_PROFILE ]] && p10k segment -f yellow -t $GITID_PROFILE }
```

Then add `gitid` to your `POWERLEVEL9K_LEFT_PROMPT_ELEMENTS` (or the right-side array, if you prefer).

## Any other prompt

`GITID_PROFILE` is a plain environment variable, so the same pattern works anywhere — a raw `PS1`, fish's `fish_prompt`, oh-my-posh, and so on. Test for the variable, print it when set. For scripting, `gitid current --format name` prints the same answer without needing the hook, and `gitid current --quiet` exits 0/1 depending on whether a profile is active.
