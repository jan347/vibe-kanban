#!/usr/bin/env bash
# scripts/bootstrap-friction.sh — one-command setup for the 3-day friction-log discipline (DX-AUTO-1).
# Idempotent: safe to re-run.

set -euo pipefail

# Fail loud on non-macOS — launchctl/osascript are macOS-only.
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: unsupported platform $(uname -s)" >&2
  echo "because: launchctl + osascript are macOS-only and the gencap CLI relies on both" >&2
  echo "try: run this experiment from a macOS box, or stub the launchd nudge step manually" >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SLUG_DIR="$HOME/.gstack/projects/jan347-vibe-kanban"

# 1. ~/bin in PATH
mkdir -p "$HOME/bin"
case ":$PATH:" in
  *":$HOME/bin:"*) ;;
  *)
    echo "WARN: ~/bin not on PATH" >&2
    echo "  add: echo 'export PATH=\"\$HOME/bin:\$PATH\"' >> ~/.zshrc" >&2
    ;;
esac

# 2. Project dir + scripts
mkdir -p "$SLUG_DIR/scripts"

# 3. friction-log.jsonl seed (only if missing — preserves existing entries on re-run)
if [[ ! -f "$SLUG_DIR/friction-log.jsonl" ]]; then
  : > "$SLUG_DIR/friction-log.jsonl"
fi

# 4. Experiment-start marker (only if missing)
if [[ ! -f "$SLUG_DIR/.experiment-start" ]]; then
  date -u +%Y-%m-%d > "$SLUG_DIR/.experiment-start"
fi

# 5. Install gencap dispatcher + per-subcommand scripts
install -m 755 "$REPO_ROOT/scripts/gencap.sh"          "$HOME/bin/gencap"
install -m 755 "$REPO_ROOT/scripts/gencap-log.sh"      "$SLUG_DIR/scripts/log.sh"
install -m 755 "$REPO_ROOT/scripts/gencap-status.sh"   "$SLUG_DIR/scripts/status.sh"
install -m 755 "$REPO_ROOT/scripts/gencap-teardown.sh" "$SLUG_DIR/scripts/teardown.sh"
install -m 755 "$REPO_ROOT/scripts/gencap-extend.sh"   "$SLUG_DIR/scripts/extend.sh"

# 6. Load launchd nudge (idempotent: unload-then-load).
PLIST_SRC="$REPO_ROOT/scripts/com.gencap.friction-nudge.plist"
PLIST_DST="$HOME/Library/LaunchAgents/com.gencap.friction-nudge.plist"
mkdir -p "$HOME/Library/LaunchAgents"
install -m 644 "$PLIST_SRC" "$PLIST_DST"
launchctl unload "$PLIST_DST" 2>/dev/null || true
launchctl load   "$PLIST_DST"

# 7. Done — show first command
echo "✓ bootstrap complete"
echo "→ Try it now:        gencap log --quick \"first friction log entry\""
echo "→ Status anytime:    gencap status"
echo "→ When done (day 4): gencap teardown"
