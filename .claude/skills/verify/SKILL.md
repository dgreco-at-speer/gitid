---
name: verify
description: Build and drive the gitid CLI end-to-end in an isolated environment to verify changes at the real surface.
---

# Verifying gitid changes

Build/test only inside the nix dev shell: `nix develop .#ci -c cargo build`.
The binary lands at `target/debug/gitid`.

## Isolated drive

Never run against your real HOME — gitid mutates the global gitconfig. Use a
scratch HOME with the same env the integration tests use:

```bash
S=$(mktemp -d /tmp/gitidv.XXXX)   # short path: unix sockets cap at ~107 bytes
mkdir -p $S/home
run() { env -i HOME=$S/home PATH=$PATH \
    GITID_CONFIG_DIR=$S/home/.config/gitid \
    GITID_DATA_DIR=$S/home/.local/share/gitid \
    GIT_CONFIG_GLOBAL=$S/home/.gitconfig GIT_CONFIG_NOSYSTEM=1 "$@"; }
run target/debug/gitid add work --non-interactive --git-name Jane --email j@x.example --no-gh
```

`env -i` matters: your own shell has GITID_PROFILE/GITID_STATE/SSH_AUTH_SOCK
set (this repo is gitid-managed), and they leak into assertions.

## Flows worth driving

- `add` → inspect `$S/home/.config/gitid/profiles.toml` and the fragment under
  `$S/home/.local/share/gitid/profiles/`.
- `use <profile> <dir>` → `git init` a repo under the dir, then
  `run git -C <repo> config <key>` to see what git actually resolves.
- Signing end-to-end: set `commit.gpgsign`, make a commit, then
  `git cat-file commit HEAD` — look for the `gpgsig` block.
- `doctor` — exit code + findings text.

## ssh-agent-held keys

Spawn a throwaway agent (short socket path!):

```bash
ssh-agent -D -a $S/agent.sock & sleep 0.3
ssh-keygen -q -t ed25519 -N "" -C "j@x.example" -f $S/id && \
  SSH_AUTH_SOCK=$S/agent.sock ssh-add $S/id && rm $S/id  # key lives only in the agent
```

Pass `SSH_AUTH_SOCK=$S/agent.sock` into `run` for agent-dependent commands
(`add --ssh-agent-key`, `sync`, `doctor`). Kill the agent to exercise the
degraded paths (sync keeps the cached `<data>/ssh/<name>.pub` and warns).
