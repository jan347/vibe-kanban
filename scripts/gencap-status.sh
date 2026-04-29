#!/usr/bin/env bash
# scripts/gencap-status.sh — current experiment state.
# Installed as ~/.gstack/projects/jan347-vibe-kanban/scripts/status.sh by bootstrap-friction.sh.

set -euo pipefail

SLUG_DIR="$HOME/.gstack/projects/jan347-vibe-kanban"
LOG_FILE="$SLUG_DIR/friction-log.jsonl"
EVENT_FILE="$SLUG_DIR/events.jsonl"
START_FILE="$SLUG_DIR/.experiment-start"

if [[ ! -d "$SLUG_DIR" ]]; then
  echo "error: $SLUG_DIR does not exist" >&2
  echo "because: bootstrap-friction.sh has not been run for this repo yet" >&2
  echo "try: cd /path/to/vibe-kanban && bash scripts/bootstrap-friction.sh" >&2
  exit 1
fi

# Resolve experiment start (default: today if marker missing).
if [[ -f "$START_FILE" ]]; then
  START_DATE="$(cat "$START_FILE" | tr -d '[:space:]')"
else
  START_DATE="$(date -u +%Y-%m-%d)"
fi

today_epoch="$(date -u -j -f '%Y-%m-%d' "$(date -u +%Y-%m-%d)" '+%s' 2>/dev/null || date -u -d "$(date -u +%Y-%m-%d)" '+%s')"
start_epoch="$(date -u -j -f '%Y-%m-%d' "$START_DATE" '+%s' 2>/dev/null || date -u -d "$START_DATE" '+%s')"
day_n=$(( (today_epoch - start_epoch) / 86400 + 1 ))
(( day_n < 1 )) && day_n=1

# Counts by severity.
count_severity() {
  local sev="$1"
  if [[ ! -f "$LOG_FILE" ]]; then echo 0; return; fi
  if command -v jq >/dev/null 2>&1; then
    jq -r --arg s "$sev" 'select(.severity == $s) | .severity' "$LOG_FILE" 2>/dev/null | wc -l | tr -d ' '
  else
    grep -c "\"severity\":\"$sev\"" "$LOG_FILE" 2>/dev/null || echo 0
  fi
}

P1_COUNT="$(count_severity P1)"
P2_COUNT="$(count_severity P2)"
P3_COUNT="$(count_severity P3)"
TOTAL_ENTRIES=$(( P1_COUNT + P2_COUNT + P3_COUNT ))

EVENT_COUNT=0
if [[ -f "$EVENT_FILE" ]]; then
  EVENT_COUNT="$(wc -l < "$EVENT_FILE" | tr -d ' ')"
fi

# Last entry timestamp.
last_ts() {
  local f="$1"
  [[ -f "$f" && -s "$f" ]] || { echo "—"; return; }
  if command -v jq >/dev/null 2>&1; then
    tail -n 1 "$f" | jq -r '.ts // "—"' 2>/dev/null || echo "—"
  else
    tail -n 1 "$f" | grep -oE '"ts":"[^"]+"' | head -n1 | sed 's/"ts":"\(.*\)"/\1/' || echo "—"
  fi
}
LAST_ENTRY_TS="$(last_ts "$LOG_FILE")"
LAST_EVENT_TS="$(last_ts "$EVENT_FILE")"

# Time until day-3 09:00 reread.
deadline_date="$(date -u -j -v +3d -f '%Y-%m-%d' "$START_DATE" '+%Y-%m-%d' 2>/dev/null \
  || date -u -d "$START_DATE +3 days" '+%Y-%m-%d')"
deadline_epoch="$(date -u -j -f '%Y-%m-%d %H:%M' "$deadline_date 09:00" '+%s' 2>/dev/null \
  || date -u -d "$deadline_date 09:00:00" '+%s')"
now_epoch="$(date -u +%s)"
secs_until=$(( deadline_epoch - now_epoch ))
if (( secs_until > 0 )); then
  hrs=$(( secs_until / 3600 ))
  mins=$(( (secs_until % 3600) / 60 ))
  TIME_UNTIL="${hrs}h ${mins}m (deadline ${deadline_date} 09:00 UTC)"
else
  TIME_UNTIL="PASSED — day-3 reread is overdue (deadline ${deadline_date} 09:00 UTC)"
fi

cat <<EOF
gencap experiment status
────────────────────────
Day:                 ${day_n} of 3   (started ${START_DATE} UTC)
Time until reread:   ${TIME_UNTIL}

friction-log.jsonl:  ${TOTAL_ENTRIES} entries
                       P1 (urgent):       ${P1_COUNT}
                       P2 (productivity): ${P2_COUNT}
                       P3 (annoyance):    ${P3_COUNT}
                     last entry: ${LAST_ENTRY_TS}

events.jsonl:        ${EVENT_COUNT} events
                     last event: ${LAST_EVENT_TS}

Kill switch:         GENCAP_FRICTION_ENABLED=${GENCAP_FRICTION_ENABLED:-1}
EOF
