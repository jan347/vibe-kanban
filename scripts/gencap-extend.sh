#!/usr/bin/env bash
# scripts/gencap-extend.sh — mid-experiment extension.
# Installed as ~/.gstack/projects/jan347-vibe-kanban/scripts/extend.sh by bootstrap-friction.sh.

set -euo pipefail

SLUG_DIR="$HOME/.gstack/projects/jan347-vibe-kanban"
START_FILE="$SLUG_DIR/.experiment-start"
CLAUDE_MD="/Users/jena/vibe-kanban/CLAUDE.md"

DAYS=""
while (( $# > 0 )); do
  case "$1" in
    --days)
      DAYS="${2:-}"
      shift 2 || true
      ;;
    -h|--help)
      cat <<'EOF'
gencap extend — extend experiment by N days

Usage:
  gencap extend --days N    Add N days (1-7) to the experiment deadline.

Updates the .experiment-start derived deadline and rewrites the CLAUDE.md banner
end-date in place.
EOF
      exit 0
      ;;
    *)
      echo "error: unknown argument \"$1\"" >&2
      echo "because: gencap extend only accepts --days N" >&2
      echo "try: gencap extend --days 2" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$DAYS" ]]; then
  echo "error: --days N is required" >&2
  echo "because: extend needs to know how many days to add to the deadline" >&2
  echo "try: gencap extend --days 2" >&2
  exit 1
fi

if ! [[ "$DAYS" =~ ^[1-7]$ ]]; then
  echo "error: invalid --days value \"$DAYS\"" >&2
  echo "because: extend accepts integers 1 through 7 (longer extensions defeat the discipline)" >&2
  echo "try: gencap extend --days 2" >&2
  exit 1
fi

if [[ ! -d "$SLUG_DIR" ]]; then
  echo "error: $SLUG_DIR does not exist" >&2
  echo "because: bootstrap-friction.sh has not been run for this repo yet" >&2
  echo "try: cd /path/to/vibe-kanban && bash scripts/bootstrap-friction.sh" >&2
  exit 1
fi

# Resolve start date — create marker with today's date if missing.
if [[ -f "$START_FILE" ]]; then
  START_DATE="$(cat "$START_FILE" | tr -d '[:space:]')"
else
  START_DATE="$(date -u +%Y-%m-%d)"
  echo "$START_DATE" > "$START_FILE"
fi

# Recompute deadline = start + 3 + N days (3 = original window, N = extension).
TOTAL_DAYS=$(( 3 + DAYS ))
NEW_DEADLINE="$(date -u -j -v +${TOTAL_DAYS}d -f '%Y-%m-%d' "$START_DATE" '+%Y-%m-%d' 2>/dev/null \
  || date -u -d "$START_DATE +${TOTAL_DAYS} days" '+%Y-%m-%d')"

# Update CLAUDE.md banner end-date in place.
# Banner line shape: > **🧪 ACTIVE EXPERIMENT (YYYY-MM-DD → YYYY-MM-DD):** ...
if [[ -f "$CLAUDE_MD" ]]; then
  sed -i '' -E "s/(\*\*🧪 ACTIVE EXPERIMENT \([0-9]{4}-[0-9]{2}-[0-9]{2} → )[0-9]{4}-[0-9]{2}-[0-9]{2}(\):\*\*)/\1${NEW_DEADLINE}\2/" "$CLAUDE_MD"
fi

echo "✓ Extended ${DAYS} days; new deadline ${NEW_DEADLINE}"
