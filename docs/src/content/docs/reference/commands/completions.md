---
title: gitid completions
description: Print tab-completion definitions for your shell.
sidebar:
  order: 15
  label: completions
---

Prints tab-completion definitions for gitid's commands and flags to stdout. Redirect the output to wherever your shell loads completions from.

## Synopsis

```sh
gitid completions <SHELL>
```

## Options

| Flag | Description |
| --- | --- |
| `<SHELL>` | Required. One of `bash`, `zsh`, `fish`, `powershell` (alias `pwsh`), or `nu` (alias `nushell`). |

## Examples

**bash** — write to your user completions directory (or eval from `~/.bashrc`):

```sh
mkdir -p ~/.local/share/bash-completion/completions
gitid completions bash > ~/.local/share/bash-completion/completions/gitid
```

**zsh** — write an `_gitid` function file onto your `fpath`:

```sh
mkdir -p ~/.zfunc
gitid completions zsh > ~/.zfunc/_gitid
# in ~/.zshrc, before compinit:
#   fpath+=(~/.zfunc)
#   autoload -Uz compinit && compinit
```

**fish** — fish picks the file up automatically:

```sh
gitid completions fish > ~/.config/fish/completions/gitid.fish
```

**PowerShell** — load from your profile:

```powershell
gitid completions powershell | Out-String | Invoke-Expression
```

**nushell** — save into the vendor autoload directory:

```sh
gitid completions nu | save -f ($nu.data-dir | path join vendor/autoload/gitid-completions.nu)
```

## Notes

- Completions describe the CLI of the binary that generated them; regenerate the file after [updating gitid](../update/) to pick up new commands or flags.
- Completions are separate from the activation hook — install that with [`gitid setup`](../setup/).

## See also

- [`gitid setup`](../setup/) — install the directory-change hook.
- [`gitid hook`](../hook/) — print the hook script itself.
