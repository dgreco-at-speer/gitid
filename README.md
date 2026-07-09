# gitid

Switch between git identities. Name/email, SSH key, commit signing, and GitHub
CLI auth — automatically, per directory tree. Assign a profile to a directory
**from anywhere**, without `cd`-ing there and without touching any repo's local
config.

![gitid demo — add two identities, map them to directory trees, and the right one applies automatically as you move between repos](demo/gitid.gif)

```console
$ gitid add work --git-name "Jane Doe" --email jane@corp.example --ssh-key ~/.ssh/id_work
$ gitid use work ~/code/work
$ cd ~/code/work/anything && git config user.email
jane@corp.example
```

> Full documentation lives in [`docs/`](docs/) — run `task docs:dev` to preview
> it locally.

## How it works

gitid never edits your repositories. Instead it leans on two mechanisms:

1. **Git conditional includes** carry the identity. gitid appends a single
   `[include]` line to your global gitconfig, pointing at a manifest it owns.
   That manifest contains one `includeIf "gitdir:…"` per mapped directory,
   each pointing at a generated per-profile fragment (`user.name`, `user.email`,
   `core.sshCommand`, signing keys, …). Git applies the right identity to every
   repo under a mapped tree — no per-repo configuration, no environment needed.

2. **A shell hook** carries everything git config cannot — primarily an isolated
   `GH_CONFIG_DIR` so each profile has its own `gh` (GitHub CLI) auth, plus a
   `GITID_PROFILE` variable for your prompt. The hook runs on directory change,
   resolves the directory→profile mapping, and exports (or restores) the right
   variables.

Your global gitconfig is touched exactly once (an append at end of file); every
other file gitid uses is generated and fully owned by gitid, so your hand-written
config and comments are never rewritten.

### File layout

```
~/.config/gitid/profiles.toml      # source of truth — edit freely, then `gitid sync`
~/.config/gitid/mappings.toml       # directory → profile assignments (machine-owned)
~/.local/share/gitid/include.gitconfig        # generated manifest of includeIf directives
~/.local/share/gitid/profiles/<name>.gitconfig # generated per-profile fragment
~/.local/share/gitid/gh/<name>/      # per-profile GH_CONFIG_DIR (gh tokens live here)
```

(Paths follow XDG on Linux/macOS and Known Folders on Windows. Override with
`GITID_CONFIG_DIR` / `GITID_DATA_DIR`.)

## Install

One-line install (replace `dgreco-at-speer/gitid` with this repository):

**Linux / macOS**

```console
$ curl -fsSL https://raw.githubusercontent.com/dgreco-at-speer/gitid/main/scripts/install.sh | sh
```

**Windows (PowerShell)**

```powershell
irm https://raw.githubusercontent.com/dgreco-at-speer/gitid/main/scripts/install.ps1 | iex
```

The script detects your OS/arch, downloads the matching release binary into
`~/.local/bin` (Windows: `%LOCALAPPDATA%\gitid\bin`), and prints the next steps.
macOS has no prebuilt binary, so the script builds from source with `cargo`.

> **Private repo?** Install the [GitHub CLI](https://cli.github.com) and run
> `gh auth login` first — the installer uses it automatically. Without `gh`,
> set `GH_TOKEN` (a token with read access) before running.

Knobs (environment variables): `GITID_REPO`, `GITID_VERSION`,
`GITID_INSTALL_DIR`.

**From source** (Rust 1.85+), e.g. for macOS or unsupported targets:

```console
$ cargo install --git https://github.com/dgreco-at-speer/gitid gitid
# or, in a checkout:
$ cargo install --path .
```

Then bootstrap and install the shell hook:

```console
$ gitid setup          # detects your shell and offers to add the hook line
```

(`gitid setup` creates the config on first run; `gitid init` does the same
explicitly.)

## Shell hook

`gitid setup` will append the right line for you, or add it manually:

| Shell | File | Line |
|-------|------|------|
| bash | `~/.bashrc` | `eval "$(gitid hook bash)"` |
| zsh | `~/.zshrc` | `eval "$(gitid hook zsh)"` |
| fish | `~/.config/fish/config.fish` | `gitid hook fish \| source` |
| PowerShell | `$PROFILE` | `Invoke-Expression (& gitid hook powershell \| Out-String)` |
| nushell | autoload dir | `gitid hook nu \| save -f ($nu.data-dir \| path join vendor/autoload/gitid.nu)` |

The hook is cheap: it only does work when the directory actually changes, and
prints nothing when the active profile is unchanged.

## Quickstart

```console
$ gitid add work --git-name "Jane Doe" --email jane@corp.example \
      --ssh-key ~/.ssh/id_work --signing ssh --signing-key ~/.ssh/id_work.pub --sign-commits
$ gitid add personal --git-name "Jane" --email jane@home.example --ssh-key ~/.ssh/id_personal

$ gitid use work ~/code/work          # assign trees from anywhere
$ gitid use personal ~/code/oss

$ gitid list                          # ● marks the profile active for your cwd
$ gitid current ~/code/work/foo       # what resolves where
$ gitid doctor                        # diagnose problems
```

Run `gitid add` with no flags for an interactive wizard (SSH-key discovery,
signing setup, optional `gh auth login`).

## Profiles reference (`profiles.toml`)

```toml
version = 1

[profiles.work]
name  = "Jane Doe"                  # → user.name
email = "jane@corp.example"         # → user.email

[profiles.work.ssh]
key = "~/.ssh/id_work"              # → core.sshCommand = "ssh -i … -o IdentitiesOnly=yes"

[profiles.work.signing]
format  = "ssh"                     # "ssh" | "openpgp"  → gpg.format
key     = "~/.ssh/id_work.pub"      # ssh: public-key path; openpgp: key id
commits = true                      # → commit.gpgsign
tags    = false                     # → tag.gpgsign (optional)

[profiles.work.gh]
enabled = true                      # provision an isolated GH_CONFIG_DIR

[profiles.work.env]                 # extra vars the hook exports (optional)
GLAB_CONFIG_DIR = "~/.config/glab-work"

[profiles.work.extra]               # raw git config passthrough (optional)
"core.autocrlf" = "input"
```

After editing by hand, run `gitid sync` to regenerate the derived files.

## Prompt integration

The hook exports `GITID_PROFILE`. Show it in your prompt:

**starship** (`~/.config/starship.toml`):

```toml
[env_var.GITID_PROFILE]
format = "[$env_value]($style) "
style = "bold yellow"
```

**powerlevel10k**:

```zsh
function prompt_gitid() { [[ -n $GITID_PROFILE ]] && p10k segment -f yellow -t $GITID_PROFILE }
# add 'gitid' to POWERLEVEL9K_LEFT_PROMPT_ELEMENTS
```

## Caveats

- **Local overrides win.** A `user.email` set in a repo's own `.git/config`
  beats any include. `gitid current` and `gitid doctor` detect and report this.
- **Worktrees.** A linked worktree resolves by the main repo's git dir. If a
  worktree lives under a mapped tree but its main repo does not, the profile
  won't apply; `gitid doctor` warns about this.
- **SSH agent.** gitid sets `core.sshCommand` with `-o IdentitiesOnly=yes` so the
  right key is used even when an agent holds several. It does **not** switch
  `SSH_AUTH_SOCK`.
- **`GIT_SSH_COMMAND`** is intentionally never exported — an env var would
  override `core.sshCommand` for every repo in the shell, defeating the design.
- Requires git ≥ 2.13. Windows is best-effort (PowerShell hook provided; CI
  covers Linux/macOS).

## Commands

| Command | Purpose |
|---------|---------|
| `gitid add [name]` | Create a profile (wizard or flags) |
| `gitid list` | List profiles (`●` = active for cwd) |
| `gitid show <name>` | Show a profile's details |
| `gitid edit <name>` | Change a profile's fields |
| `gitid remove <name>` | Delete a profile |
| `gitid use <profile> [dir]` | Assign a profile to a directory tree |
| `gitid forget [dir]` | Remove a directory mapping |
| `gitid dirs` | List directory→profile mappings |
| `gitid current [dir]` | Show the profile active for a directory |
| `gitid doctor [dir]` | Diagnose configuration problems |
| `gitid sync` | Regenerate derived files from the stores |
| `gitid env --shell <s>` | Print activation for a directory (used by hooks) |
| `gitid hook <shell>` | Print the shell hook script |
| `gitid setup [shell]` | Install the hook into your rc file |
| `gitid mcp serve` | Run the MCP server over stdio (for agent harnesses) |
| `gitid mcp install [client…]` | Register the MCP server in agent harness configs |
| `gitid completions <shell>` | Print shell completions |
| `gitid update` | Update gitid to the latest release |
| `gitid update --check` | Check for a newer release without installing |

## Updating

`gitid update` downloads the latest release and replaces the running binary in
place (it swaps whichever `gitid` is executing, wherever it lives). `gitid update
--check` only reports whether a newer version exists.

gitid also checks for updates opportunistically: at most once a day, in the
background, during normal commands. When a newer version is known it prints a
one-line notice to stderr (never to stdout, so pipelines are unaffected) — it
never installs anything on its own. The background check only runs in interactive
sessions, and shell-eval commands (`env`, `hook`, `completions`) stay silent.

Because releases live in a private repo, updating needs auth the same way the
installer does: install the [GitHub CLI](https://cli.github.com) and run `gh auth
login`, or set `GH_TOKEN`. macOS has no prebuilt binary yet, so `update` there
points you at `cargo install` until darwin releases exist.

Knobs (environment variables): `GITID_REPO`, `GITID_VERSION` (pin a specific tag),
`GITID_NO_UPDATE_CHECK` (disable the background check entirely), and
`GITID_UPDATE_INTERVAL` (override the check interval, in seconds).

## MCP server

gitid ships a [Model Context Protocol](https://modelcontextprotocol.io) server so
agents can drive it natively (instead of shelling out and parsing text). It is a
stdio server: the harness spawns `gitid mcp serve` as a child process per session
— there is no daemon or socket to manage.

Register it once, globally, with:

```sh
gitid mcp install                 # all supported harnesses
gitid mcp install claude-code     # or name specific ones: claude-code, cursor, opencode
gitid mcp install cursor --project   # write the project-local config instead of the user config
gitid mcp install opencode --print   # show what would be written, change nothing
```

`install` merges a `gitid` entry into each harness's MCP config (`~/.claude.json`,
`~/.cursor/mcp.json`, `~/.config/opencode/opencode.json`, or the `--project`
equivalents), preserving everything else, and prompts before writing (`--yes` to
skip). The server exposes read tools (`gitid_list`, `gitid_show`, `gitid_dirs`,
`gitid_current`, `gitid_doctor`) and write tools (`gitid_use`, `gitid_forget`,
`gitid_add`, `gitid_edit`, `gitid_remove`, `gitid_sync`); every mutation flows
through the same sync step as the CLI.

## For AI agents

[`skills/gitid/`](skills/gitid/SKILL.md) is a portable, LLM-agnostic Agent Skill
that teaches any agent how to drive gitid — the mental model, core workflow, and
how to diagnose "commits show the wrong identity". Use it with Claude Code / the
Agent SDK (symlink into `~/.claude/skills/`), reference it from another agent's
rules, or paste `SKILL.md` into any chat LLM. See [`skills/README.md`](skills/README.md).

Agents that speak MCP can instead use the built-in server above (`gitid mcp
install`), which exposes the same operations as typed tools.
