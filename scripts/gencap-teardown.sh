#!/usr/bin/env bash
# scripts/gencap-teardown.sh — day-4 cleanup automation (DX-AUTO-8).
# Installed as ~/.gstack/projects/jan347-vibe-kanban/scripts/teardown.sh by bootstrap-friction.sh.
# Idempotent: silent on already-removed pieces.

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: unsupported platform $(uname -s)" >&2
  echo "because: launchctl is macOS-only and the teardown unloads a launchd plist" >&2
  echo "try: run this on macOS, or skip this step and remove ~/Library/LaunchAgents/com.gencap.* manually" >&2
  exit 1
fi

DATE="$(date -u +%Y-%m-%d)"
SLUG_DIR="$HOME/.gstack/projects/jan347-vibe-kanban"
PLIST="$HOME/Library/LaunchAgents/com.gencap.friction-nudge.plist"
CLAUDE_MD="/Users/jena/vibe-kanban/CLAUDE.md"

# 1. Stop launchd nudge (silent if already absent).
launchctl unload "$PLIST" 2>/dev/null || true
rm -f "$PLIST"

# 2. Archive friction-log.jsonl (silent if absent).
if [[ -f "$SLUG_DIR/friction-log.jsonl" ]]; then
  mv "$SLUG_DIR/friction-log.jsonl" "$SLUG_DIR/friction-log-archived-${DATE}.jsonl"
fi

# 3. Archive events.jsonl (silent if absent).
if [[ -f "$SLUG_DIR/events.jsonl" ]]; then
  mv "$SLUG_DIR/events.jsonl" "$SLUG_DIR/events-archived-${DATE}.jsonl"
fi

# 4. Remove experiment banner from CLAUDE.md (silent if banner already absent).
if [[ -f "$CLAUDE_MD" ]]; then
  # Use sed -i '' for BSD/macOS sed; range deletes the banner block.
  sed -i '' '/^> \*\*🧪 ACTIVE EXPERIMENT/,/^> \*\*Schema:/d' "$CLAUDE_MD"
fi

echo "✓ Experiment archived"
echo "  - launchd nudge unloaded + plist removed"
echo "  - friction-log.jsonl → friction-log-archived-${DATE}.jsonl (if existed)"
echo "  - events.jsonl → events-archived-${DATE}.jsonl (if existed)"
echo "  - CLAUDE.md banner removed (if present)"
