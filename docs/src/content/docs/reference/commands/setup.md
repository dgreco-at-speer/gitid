---
title: gitid setup
description: Install the gitid shell hook into your shell's rc file.
sidebar:
  order: 14
  label: setup
---

Installs the shell hook line into your shell's rc file, asking for confirmation first.

## Synopsis

```sh
gitid setup [SHELL] [--print] [--yes]
```

## Options

| Flag | Description |
| --- | --- |
| `[SHELL]` | Shell to set up: `bash`, `zsh`, `fish`, `powershell` (alias `pwsh`), or `nu` (alias `nushell`). Auto-detected from `$SHELL` when omitted (PowerShell is assumed on Windows). |
| `--print` | Only print the line to add and which file it belongs in; do not modify anything. |
| `-y, --yes` | Append to the rc file without prompting. Required when stdin is not a terminal (scripts, CI). |

## Which file it edits

| Shell | rc file |
| --- | --- |
| bash | `~/.bashrc` |
| zsh | `~/.zshrc` |
| fish | `~/.config/fish/config.fish` |
| PowerShell | `~/.config/powershell/Microsoft.PowerShell_profile.ps1` |
| nushell | `~/.config/nushell/vendor/autoload/gitid.nu` |

For bash/zsh/fish/PowerShell, setup appends the hook line wrapped in `# >>> gitid hook >>>` / `# <<< gitid hook <<<` markers. For nushell — which cannot `eval` — it writes the full hook script to a standalone autoload file instead.

## Idempotency

Running `gitid setup` again is safe: if the marker block is already present it reports "hook already installed" and changes nothing. Parent directories are created if needed, and existing rc content is never rewritten — only appended to.

## Examples

Interactive install with shell auto-detection:

```console
$ gitid setup
ℹ will append the gitid hook to /home/jane/.zshrc:
    eval "$(gitid hook zsh)"
? Append to /home/jane/.zshrc? (y/N) y
✔ installed gitid hook in /home/jane/.zshrc
ℹ restart your shell (or source the rc file) to activate
```

Just show what would be added, without touching anything:

```console
$ gitid setup fish --print
# add to /home/jane/.config/fish/config.fish
gitid hook fish | source
```

Non-interactive install (dotfiles scripts, CI):

```sh
gitid setup zsh --yes
```

## Notes

- Declining the prompt is fine — setup prints the exact line so you can add it wherever you manage your dotfiles.
- Restart your shell (or source the rc file) after installing; the hook activates the profile for your current directory immediately on load.

## See also

- [`gitid hook`](../hook/) — the script the installed line evaluates.
- [`gitid doctor`](../doctor/) — verifies the hook is installed.
- [Shell integration guide](../../../guides/shell-integration/)
