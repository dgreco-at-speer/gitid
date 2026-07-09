#!/usr/bin/env bash
# Hermetic sandbox for the VHS demo recording (demo/gitid.tape).
#
# Sourced (hidden) at the top of the tape so the recorded commands run against a
# throwaway HOME instead of your real one. gitid resolves all of its state from
# $HOME, $GITID_CONFIG_DIR, $GITID_DATA_DIR and $GH_CONFIG_DIR (see src/paths.rs),
# so redirecting those four keeps your real ~/.gitconfig and ~/.config/gitid
# untouched. Intended to be `source`d, not executed — so no `set -e`, which would
# kill the interactive shell VHS is driving.

# The freshly built binary takes precedence on PATH.
export PATH="$PWD/target/release:$PATH"

# Never phone home for an update check mid-recording (would print to stderr).
export GITID_NO_UPDATE_CHECK=1

# Throwaway HOME. A fixed, clean path (deliberately not $TMPDIR — under `nix
# develop` that is a long `/tmp/nix-shell.*` path that would leak into the dir
# names `gitid use`/`gitid dirs` echo on screen). Wiped at start and on exit so
# each render is reproducible.
export HOME="/tmp/gitid-demo"
rm -rf "$HOME"
export GITID_CONFIG_DIR="$HOME/.config/gitid"
export GITID_DATA_DIR="$HOME/.local/share/gitid"
export GH_CONFIG_DIR="$HOME/.config/gh"
trap 'rm -rf "$HOME"' EXIT

mkdir -p "$HOME/.ssh" "$HOME/code/work/app" "$HOME/code/oss/lib"

# Fake keys so the demo's --ssh-key paths resolve cleanly.
ssh-keygen -q -t ed25519 -N '' -C jane@corp.example -f "$HOME/.ssh/id_work"
ssh-keygen -q -t ed25519 -N '' -C jane@home.example -f "$HOME/.ssh/id_personal"

# Minimal git + repos inside the sandbox so the `cd … && git config user.email`
# beat has real repositories to resolve an identity in.
git config --global init.defaultBranch main
git init -q "$HOME/code/work/app"
git init -q "$HOME/code/oss/lib"

# Bootstrap gitid's config dirs and the global include hook inside the sandbox.
gitid init >/dev/null 2>&1

# Clean, stable prompt for the recording.
export PS1='$ '
clear
