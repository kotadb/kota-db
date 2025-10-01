#!/usr/bin/env bash
set -euo pipefail

HOOK_DIR="$(git rev-parse --show-toplevel)/.git/hooks"
SCRIPT_DIR="$(git rev-parse --show-toplevel)/hooks"

echo "Installing git hooks from ${SCRIPT_DIR} -> ${HOOK_DIR}" >&2
mkdir -p "$HOOK_DIR"
for hook in prepare-commit-msg; do
  src="$SCRIPT_DIR/$hook"
  dest="$HOOK_DIR/$hook"
  if [ ! -f "$src" ]; then
    echo "Missing hook template: $src" >&2
    exit 1
  fi
  cp "$src" "$dest"
  chmod +x "$dest"
  echo "Installed $hook hook" >&2
done

echo "Git hooks installed." >&2
