#!/usr/bin/env bash
# scripts/gencap.sh — top-level dispatcher for the gencap CLI (DX-AUTO-2 + DX-AUTO-7).
# Installed as ~/bin/gencap by bootstrap-friction.sh.

set -euo pipefail

SLUG_DIR="$HOME/.gstack/projects/jan347-vibe-kanban"
SCRIPTS="$SLUG_DIR/scripts"

usage() {
  cat <<'EOF'
gencap — friction-log discipline CLI for the GenCap Control Room experiment

Usage:
  gencap <subcommand> [args]

Subcommands:
  log [--quick "msg"]   Append a friction-log entry (interactive prompts; --quick for terse).
  status                Show experiment status: day N of 3, last entry/event timestamps,
                        counts by severity, time until day-3 reread.
  extend --days N       Mid-experiment extension: shifts day-3 deadline by N days
                        and updates the CLAUDE.md banner.
  teardown              Day-4 cleanup: launchctl unload, archive friction-log.jsonl
                        + events.jsonl with date stamp, remove CLAUDE.md banner.
  --help | help         Show this usage.

Environment:
  GENCAP_FRICTION_ENABLED=0   Kill switch — emitter no-ops (no rebuild needed).

Examples:
  gencap log                              # interactive prompt
  gencap log --quick "model retry storm"  # 1-line capture
  gencap status                           # current state
  gencap extend --days 2                  # add 2 days
  gencap teardown                         # end experiment
EOF
}

bootstrap_hint() {
  echo "error: gencap subcommand script not found at $SCRIPTS" >&2
  echo "because: bootstrap-friction.sh has not been run for this repo yet" >&2
  echo "try: cd /path/to/vibe-kanban && bash scripts/bootstrap-friction.sh" >&2
  exit 1
}

case "${1:-}" in
  log)
    shift
    [[ -x "$SCRIPTS/log.sh" ]] || bootstrap_hint
    exec "$SCRIPTS/log.sh" "$@"
    ;;
  status)
    shift
    [[ -x "$SCRIPTS/status.sh" ]] || bootstrap_hint
    exec "$SCRIPTS/status.sh" "$@"
    ;;
  teardown)
    shift
    [[ -x "$SCRIPTS/teardown.sh" ]] || bootstrap_hint
    exec "$SCRIPTS/teardown.sh" "$@"
    ;;
  extend)
    shift
    [[ -x "$SCRIPTS/extend.sh" ]] || bootstrap_hint
    exec "$SCRIPTS/extend.sh" "$@"
    ;;
  --help|help|"")
    usage
    ;;
  *)
    echo "error: unknown subcommand \"$1\"" >&2
    echo "because: gencap only knows: log, status, extend, teardown" >&2
    echo "try: gencap --help" >&2
    exit 1
    ;;
esac
