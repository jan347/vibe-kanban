#!/usr/bin/env bash
# scripts/gencap-log.sh — friction-log entry capture (D-AUTO-3 + DX-AUTO-3 + DX-AUTO-7).
# Installed as ~/.gstack/projects/jan347-vibe-kanban/scripts/log.sh by bootstrap-friction.sh.
#
# Writes ONE valid JSON line per invocation to friction-log.jsonl, using flock(1) for
# cross-process safety (matches the planned Rust emitter pattern from E-AUTO-2 Option A).

set -euo pipefail

SLUG_DIR="$HOME/.gstack/projects/jan347-vibe-kanban"
LOG_FILE="$SLUG_DIR/friction-log.jsonl"
LOCK_FILE="$SLUG_DIR/.friction-log.lock"

VALID_VENTURES=("chief-of-staff" "carbonv3" "fultech" "port-analytics" "other")
VALID_LAYERS=("cockpit-ui" "cockpit-coord" "agent" "external")
VALID_SEVERITIES=("P1" "P2" "P3")

# --- DX-AUTO-7 kill switch ---
if [[ "${GENCAP_FRICTION_ENABLED:-1}" == "0" ]]; then
  # No-op: emitter disabled. Print nothing on stdout to keep pipelines clean; emit a
  # one-line stderr trace so the user can confirm the kill switch is in effect.
  echo "gencap log: GENCAP_FRICTION_ENABLED=0 — entry not written" >&2
  exit 0
fi

# --- bootstrap check ---
if [[ ! -d "$SLUG_DIR" ]]; then
  echo "error: $SLUG_DIR does not exist" >&2
  echo "because: bootstrap-friction.sh has not been run for this repo yet" >&2
  echo "try: cd /path/to/vibe-kanban && bash scripts/bootstrap-friction.sh" >&2
  exit 1
fi

# --- arg parsing ---
QUICK_MSG=""
SEVERITY_ARG=""
while (( $# > 0 )); do
  case "$1" in
    --quick)
      QUICK_MSG="${2:-}"
      if [[ -z "$QUICK_MSG" ]]; then
        echo "error: --quick requires a message argument" >&2
        echo "because: --quick captures a single terse line; empty messages are not useful" >&2
        echo "try: gencap log --quick \"model retry storm\"" >&2
        exit 1
      fi
      shift 2
      ;;
    --severity)
      SEVERITY_ARG="${2:-}"
      if [[ ! "$SEVERITY_ARG" =~ ^P[123]$ ]]; then
        echo "error: invalid --severity \"$SEVERITY_ARG\"" >&2
        echo "because: severity must be one of: P1, P2, P3" >&2
        echo "try: gencap log --quick \"...\" --severity P2" >&2
        exit 1
      fi
      shift 2
      ;;
    -h|--help)
      cat <<'EOF'
gencap log — append a friction-log entry

Usage:
  gencap log                              Interactive prompts (venture/layer/severity/details).
  gencap log --quick "msg"                Terse 1-line capture (defaults venture=other, layer=cockpit-ui).
                                          Severity prompt still required UNLESS stdin is a tty.
  gencap log --quick "msg" --severity P2  Fully non-interactive (script-friendly).

Each invocation writes ONE valid JSON line to ~/.gstack/projects/jan347-vibe-kanban/friction-log.jsonl.
EOF
      exit 0
      ;;
    *)
      echo "error: unknown argument \"$1\"" >&2
      echo "because: gencap log accepts --quick \"msg\" [--severity P1|P2|P3] (or no args for interactive)" >&2
      echo "try: gencap log --help" >&2
      exit 1
      ;;
  esac
done

# --- helpers ---
contains() {
  local needle="$1"; shift
  local item
  for item in "$@"; do
    [[ "$item" == "$needle" ]] && return 0
  done
  return 1
}

prompt_enum() {
  local label="$1"; shift
  local -a allowed=("$@")
  local val
  local options
  options="$(IFS=/; printf '%s' "${allowed[*]}")"
  # Non-tty stdin → fail fast instead of infinite-looping on empty reads.
  # The original loop assumed an interactive terminal; piped or sourced
  # invocations would burn CPU forever. Real bug found during setup.
  if [[ ! -t 0 ]]; then
    echo "error: cannot prompt for $label" >&2
    echo "because: stdin is not a terminal (piped or scripted invocation)" >&2
    echo "try: run \`gencap log\` interactively, OR pass --severity P1|P2|P3 with --quick" >&2
    exit 1
  fi
  while true; do
    read -r -p "$label? [$options]: " val
    if contains "$val" "${allowed[@]}"; then
      printf '%s' "$val"
      return 0
    fi
    echo "" >&2
    echo "error: invalid $label \"$val\"" >&2
    echo "because: must be one of: $(IFS=,; echo "${allowed[*]}")" >&2
    echo "try: re-enter, or use \"other\" to provide a free-text venture name" >&2
    echo "" >&2
  done
}

prompt_text() {
  local label="$1"
  local val
  read -r -p "$label: " val
  printf '%s' "$val"
}

prompt_int() {
  local label="$1"
  local val
  while true; do
    read -r -p "$label: " val
    if [[ "$val" =~ ^[0-9]+$ ]]; then
      printf '%s' "$val"
      return 0
    fi
    echo "" >&2
    echo "error: invalid integer \"$val\"" >&2
    echo "because: $label must be a non-negative integer (minutes)" >&2
    echo "try: enter a whole number, e.g. 12" >&2
    echo "" >&2
  done
}

# JSON-escape a single string (\, ", control chars).
json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

# --- gather fields ---
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

if [[ -n "$QUICK_MSG" ]]; then
  # Quick mode (D-AUTO-3 spec): skip prompts except severity, default venture/layer.
  # If --severity passed, fully non-interactive; otherwise prompt (which
  # now fails fast on non-tty rather than looping forever).
  if [[ -n "$SEVERITY_ARG" ]]; then
    SEVERITY="$SEVERITY_ARG"
  else
    SEVERITY="$(prompt_enum 'Severity' "${VALID_SEVERITIES[@]}")"
  fi
  VENTURE="other"
  VENTURE_OTHER=""
  LAYER="cockpit-ui"
  WHAT_I_TRIED="$QUICK_MSG"
  WHAT_BLOCKED=""
  WORKAROUND=""
  TIME_LOST_MIN="0"
else
  VENTURE="$(prompt_enum 'Venture' "${VALID_VENTURES[@]}")"
  VENTURE_OTHER=""
  if [[ "$VENTURE" == "other" ]]; then
    while true; do
      read -r -p 'Venture (free text)?: ' VENTURE_OTHER
      if [[ -n "$VENTURE_OTHER" ]]; then break; fi
      echo "" >&2
      echo "error: empty free-text venture" >&2
      echo "because: \"other\" requires a non-empty venture name for slicing" >&2
      echo "try: enter a short slug, e.g. \"side-project-x\"" >&2
      echo "" >&2
    done
  fi
  LAYER="$(prompt_enum 'Layer' "${VALID_LAYERS[@]}")"
  SEVERITY="$(prompt_enum 'Severity' "${VALID_SEVERITIES[@]}")"
  WHAT_I_TRIED="$(prompt_text 'What were you trying to do?')"
  WHAT_BLOCKED="$(prompt_text 'What blocked you?')"
  WORKAROUND="$(prompt_text 'Workaround?')"
  TIME_LOST_MIN="$(prompt_int 'Time lost (minutes)?')"
fi

# --- build single-line JSON record (single buffered write_all + flock for atomicity) ---
# NOTE: bash command substitution $(...) strips trailing newlines, so the trailing
# \n MUST be added at write-time (not in LINE) — otherwise BSD `wc -l` counts 0.
LINE=$(printf '{"v":1,"ts":"%s","venture":"%s","venture_other":"%s","layer":"%s","severity":"%s","what_i_tried":"%s","what_blocked":"%s","workaround":"%s","time_lost_min":%s}' \
  "$TS" \
  "$(json_escape "$VENTURE")" \
  "$(json_escape "$VENTURE_OTHER")" \
  "$(json_escape "$LAYER")" \
  "$(json_escape "$SEVERITY")" \
  "$(json_escape "$WHAT_I_TRIED")" \
  "$(json_escape "$WHAT_BLOCKED")" \
  "$(json_escape "$WORKAROUND")" \
  "$TIME_LOST_MIN")

# Ensure parent dir exists (idempotent).
mkdir -p "$SLUG_DIR"

# flock(1) cross-process advisory lock around the append.
# Use a separate lock file so log readers don't block.
# A single printf produces one write(2) syscall for short lines; with flock
# this matches the planned Rust emitter pattern from E-AUTO-2 Option A.
if command -v flock >/dev/null 2>&1; then
  exec 9>>"$LOCK_FILE"
  flock 9
  printf '%s\n' "$LINE" >> "$LOG_FILE"
  flock -u 9
  exec 9>&-
else
  # Fallback (macOS without coreutils flock): O_APPEND single write_all.
  # Bash >> opens with O_APPEND; one printf is one write(2) call for short lines.
  printf '%s\n' "$LINE" >> "$LOG_FILE"
fi

echo "✓ Logged to friction-log.jsonl at $TS"
