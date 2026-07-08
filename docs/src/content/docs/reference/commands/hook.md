---
title: gitid hook
description: Print the shell hook script to eval or source in your shell rc file.
sidebar:
  order: 13
  label: hook
---

Prints the hook script for a shell. Evaluated in your rc file, the hook runs `gitid env` whenever the directory changes and applies its output, keeping `GITID_PROFILE` and friends in sync as you `cd` around.

## Synopsis

```sh
gitid hook <SHELL>
```

## Options

| Flag | Description |
| --- | --- |
| `<SHELL>` | Required. One of `bash`, `zsh`, `fish`, `powershell` (alias `pwsh`), or `nu` (alias `nushell`). |

## Installing the hook

[`gitid setup`](../setup/) adds the right line for you. To do it manually, add one line per shell:

| Shell | File | Line |
| --- | --- | --- |
| bash | `~/.bashrc` | `eval "$(gitid hook bash)"` |
| zsh | `~/.zshrc` | `eval "$(gitid hook zsh)"` |
| fish | `~/.config/fish/config.fish` | `gitid hook fish \| source` |
| PowerShell | `$PROFILE` | `Invoke-Expression (& gitid hook powershell \| Out-String)` |
| nushell | autoload dir | `gitid hook nu \| save -f ($nu.data-dir \| path join vendor/autoload/gitid.nu)` |

Nushell cannot `eval` dynamically, so its hook is saved as a standalone autoload file instead of being sourced from a config line.

## Examples

Inspect the bash hook before installing it:

```console
$ gitid hook bash
# gitid bash hook. Add to ~/.bashrc:  eval "$(gitid hook bash)"
_gitid_hook() {
  local _gitid_status=$?
  if [[ "${_GITID_PWD-}" != "$PWD" ]]; then
    _GITID_PWD=$PWD
    eval "$(gitid env --shell bash)"
  fi
  return $_gitid_status
}
...
```

Try the hook in the current shell session without touching any file:

```sh
eval "$(gitid hook zsh)"
```

## Notes

- The hook is cheap: it only calls `gitid env` when the directory actually changed, and `env` prints nothing when the active profile is unchanged.
- The hook runs once at load time too, so a new shell that starts inside a mapped directory activates immediately.

## See also

- [`gitid setup`](../setup/) — install the hook into your rc file automatically.
- [`gitid env`](../env/) — what the hook evaluates on each directory change.
- [Shell integration guide](../../../guides/shell-integration/)
