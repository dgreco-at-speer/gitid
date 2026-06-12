# gitid reference

Full command, schema, and layout reference. See `SKILL.md` for the mental model
and workflows.

## Commands

Global flags: `--no-color`, `-v/--verbose` (repeatable). Output is uncolored when
not a TTY or when `NO_COLOR` is set.

| Command | Args / flags | Notes |
|---|---|---|
| `add` (alias `new`) | `[NAME]` `--git-name <s>` `--email <s>` `--ssh-key <path>` `--signing ssh\|openpgp\|none` `--signing-key <val>` `--sign-commits` `--gh`/`--no-gh` `--non-interactive` | Creates a profile. In automation always pass `--non-interactive` plus `--git-name` and `--email` (required); otherwise a wizard prompts. Ends by running sync + bootstrapping the global include. |
| `list` (alias `ls`) | `--format table\|json\|names` | `●` marks the profile active for the current directory. |
| `show` | `<NAME>` `--format pretty\|json` | Includes the generated fragment path and mapped directories. |
| `edit` | `<NAME>` `--git-name` `--email` `--ssh-key` `--signing` `--signing-key` `--open` | Sets fields, then syncs. `--open` (editor) is not yet implemented — edit `profiles.toml` and run `sync` instead. |
| `remove` (alias `rm`) | `<NAME>` `-f/--force` | Refuses if directories map to it unless `--force` (which also drops those mappings). Never deletes the gh auth dir. |
| `use` | `<PROFILE> [DIR]` `--icase`/`--no-icase` | Assigns a profile to `DIR` (default: cwd). Works from anywhere; canonicalizes the path. icase defaults on Windows/macOS, off on Linux. |
| `forget` | `[DIR]` | Removes the mapping for `DIR` (default: cwd). |
| `dirs` | `--format table\|json\|names` | Lists directory→profile mappings. |
| `current` (alias `whoami`) | `[DIR]` `--format pretty\|json\|name` `-q/--quiet` | Shows the active profile for `DIR`; in a repo it cross-checks `git config user.email` and warns on mismatch. `--quiet`: exit 0 if active, 1 if not. |
| `doctor` | `[DIR]` | Diagnostics; exit 0 = healthy, 1 = problems. Probes git's real resolution in `DIR` if it is a repo. |
| `sync` | — | Regenerates all derived files from the two stores and ensures the global include. Run after hand-edits. |
| `env` | `-s/--shell <shell>` `--dir <d>` | Hot path called by the hook. Prints shell activation or nothing. Don't call manually for side effects. |
| `hook` | `<shell>` | Prints the hook script to eval/source. |
| `setup` | `[shell]` `--print` `-y/--yes` | Installs the hook into the rc file (asks first; `--print` just shows the line; `--yes` appends without prompting). |
| `completions` | `<shell>` | Prints shell completions. |
| `init` | — | Creates config/data dirs + the global include, then prints hook instructions. Optional — mutating commands bootstrap lazily. |

`<shell>` ∈ `bash`, `zsh`, `fish`, `powershell` (alias `pwsh`), `nu` (alias
`nushell`).

`add`, `edit`, `use`, `remove`, `forget` all run `sync` automatically.

## `profiles.toml` (`~/.config/gitid/profiles.toml`) — hand-editable

```toml
version = 1

[profiles.work]
name  = "Jane Doe"                  # required → user.name
email = "jane@corp.example"         # required → user.email

[profiles.work.ssh]                 # optional
key = "~/.ssh/id_work"              # → core.sshCommand = "ssh -i <abs> -o IdentitiesOnly=yes"

[profiles.work.signing]             # optional
format  = "ssh"                     # "ssh" | "openpgp"  → gpg.format
key     = "~/.ssh/id_work.pub"      # ssh: public-key path; openpgp: key id → user.signingkey
commits = true                      # → commit.gpgsign
tags    = false                     # optional → tag.gpgsign

[profiles.work.gh]                  # optional
enabled = true                      # provision an isolated GH_CONFIG_DIR (default true)

[profiles.work.env]                 # optional: extra vars the hook exports
GLAB_CONFIG_DIR = "~/.config/glab-work"

[profiles.work.extra]               # optional: raw git config keys, written verbatim
"core.autocrlf" = "input"
"url.git@github.com-work:.insteadOf" = "git@github.com:"
```

Profile names must match `^[a-z0-9][a-z0-9_-]*$` (they become filenames). `~`
paths are expanded to absolute in generated config. After hand-editing, run
`gitid sync`.

## `mappings.toml` (`~/.config/gitid/mappings.toml`) — machine-owned

```toml
version = 1
[[mapping]]
dir = "/home/u/code/work/"          # canonical, forward slashes, trailing slash
profile = "work"
case_insensitive = false
[mapping.env]                       # derived at sync time; read by `gitid env`
GH_CONFIG_DIR = "/home/u/.local/share/gitid/gh/work"
```

Prefer `gitid use`/`gitid forget` over editing this; run `gitid sync` if you do.

## File layout

```
~/.config/gitid/profiles.toml        # source of truth: identities
~/.config/gitid/mappings.toml         # source of truth: directory→profile
~/.local/share/gitid/include.gitconfig          # generated manifest of includeIf lines
~/.local/share/gitid/profiles/<name>.gitconfig  # generated per-profile fragment
~/.local/share/gitid/gh/<name>/        # per-profile GH_CONFIG_DIR (gh tokens)
~/.gitconfig                           # gets ONE appended [include] line, once
```

Paths follow XDG on Linux/macOS and Known Folders on Windows.

## Environment variables

| Variable | Meaning |
|---|---|
| `GITID_PROFILE` | Exported by the hook; the active profile name (use in prompts). |
| `GH_CONFIG_DIR` | Exported by the hook when the profile enables gh isolation. |
| `GITID_STATE` | Internal; the hook's saved-values diff. Do not set manually. |
| `GITID_DISABLE` | If set non-empty, `gitid env` emits nothing (kill switch). |
| `GITID_CONFIG_DIR` / `GITID_DATA_DIR` | Override the config/data locations (tests, custom installs). |

## Exit codes

`gitid current --quiet`: 0 if a profile is active, 1 otherwise. `gitid doctor`: 0
healthy, 1 problems found. Other commands: 0 success, nonzero on error (message
on stderr). `gitid env` always exits 0 (never breaks a prompt).
