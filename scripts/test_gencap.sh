#!/usr/bin/env bash
# scripts/test_gencap.sh — minimal shell tests for the gencap CLI.
# These tests run against the SOURCE scripts (no bootstrap install required) by
# constructing an isolated $HOME so the CLI sees a clean state every time.

set -uo pipefail   # don't -e: we want failed assertions to keep going and report

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPTS_DIR="$REPO_ROOT/scripts"

PASS=0
FAIL=0
FAILED_TESTS=()

assert_contains() {
  local label="$1" haystack="$2" needle="$3"
  if [[ "$haystack" == *"$needle"* ]]; then
    return 0
  else
    echo "  FAIL: $label"
    echo "    expected to contain: $needle"
    echo "    actual: $haystack"
    return 1
  fi
}

run_test() {
  local name="$1"; shift
  echo ""
  echo "TEST: $name"
  if "$@"; then
    PASS=$((PASS+1))
    echo "  PASS"
  else
    FAIL=$((FAIL+1))
    FAILED_TESTS+=("$name")
  fi
}

setup_sandbox() {
  # Build an isolated $HOME with the gencap layout pre-installed.
  SANDBOX="$(mktemp -d -t gencap_test.XXXXXX)"
  export HOME="$SANDBOX"
  local slug="$HOME/.gstack/projects/jan347-vibe-kanban"
  mkdir -p "$slug/scripts" "$HOME/bin" "$HOME/Library/LaunchAgents"
  : > "$slug/friction-log.jsonl"
  install -m 755 "$SCRIPTS_DIR/gencap.sh"          "$HOME/bin/gencap"
  install -m 755 "$SCRIPTS_DIR/gencap-log.sh"      "$slug/scripts/log.sh"
  install -m 755 "$SCRIPTS_DIR/gencap-status.sh"   "$slug/scripts/status.sh"
  install -m 755 "$SCRIPTS_DIR/gencap-teardown.sh" "$slug/scripts/teardown.sh"
  install -m 755 "$SCRIPTS_DIR/gencap-extend.sh"   "$slug/scripts/extend.sh"
  date -u +%Y-%m-%d > "$slug/.experiment-start"
  export PATH="$HOME/bin:$PATH"
}

teardown_sandbox() {
  [[ -n "${SANDBOX:-}" && -d "$SANDBOX" ]] && rm -rf "$SANDBOX"
}

# --- TEST 1: --help shows all 4 subcommands ---
test_help_shows_subcommands() {
  setup_sandbox
  local out
  out="$(gencap --help 2>&1 || true)"
  teardown_sandbox

  assert_contains "help has log"      "$out" "log " || return 1
  assert_contains "help has status"   "$out" "status" || return 1
  assert_contains "help has extend"   "$out" "extend" || return 1
  assert_contains "help has teardown" "$out" "teardown" || return 1
  assert_contains "help mentions kill switch" "$out" "GENCAP_FRICTION_ENABLED" || return 1
  return 0
}

# --- TEST 2: log --quick appends exactly one valid JSON line ---
test_log_quick_appends_line() {
  setup_sandbox
  local logf="$HOME/.gstack/projects/jan347-vibe-kanban/friction-log.jsonl"
  : > "$logf"
  printf 'P2\n' | gencap log --quick "test entry from harness" >/dev/null 2>&1 || true
  local lines
  lines="$(wc -l < "$logf" | tr -d ' ')"
  local first_line
  first_line="$(head -n1 "$logf")"
  teardown_sandbox

  if [[ "$lines" != "1" ]]; then
    echo "  FAIL: expected 1 line, got $lines"
    return 1
  fi
  # Sanity: line should look like JSON and contain expected fields.
  assert_contains "line has ts"           "$first_line" "\"ts\":"           || return 1
  assert_contains "line has venture"      "$first_line" "\"venture\":"      || return 1
  assert_contains "line has severity P2"  "$first_line" "\"severity\":\"P2\"" || return 1
  assert_contains "line has what_i_tried" "$first_line" "test entry from harness" || return 1
  return 0
}

# --- TEST 3: invalid venture shows error template ---
test_log_invalid_venture_shows_template() {
  setup_sandbox
  # Feed: bad venture, then good venture, then layer, severity, ... but we only need
  # to see the error template — pipe will close after first three reads.
  local out
  out="$(printf 'foo\ncarbonv3\nagent\nP3\nx\ny\nz\n0\n' | gencap log 2>&1 || true)"
  teardown_sandbox

  assert_contains "error: line"   "$out" "error:"   || return 1
  assert_contains "because: line" "$out" "because:" || return 1
  assert_contains "try: line"     "$out" "try:"     || return 1
  return 0
}

# --- TEST 4: missing slug dir shows bootstrap hint ---
test_log_missing_dir_shows_bootstrap_hint() {
  # Use a sandbox but do NOT create the slug dir — install the dispatcher only.
  SANDBOX="$(mktemp -d -t gencap_test.XXXXXX)"
  export HOME="$SANDBOX"
  mkdir -p "$HOME/bin"
  install -m 755 "$SCRIPTS_DIR/gencap.sh" "$HOME/bin/gencap"
  export PATH="$HOME/bin:$PATH"

  local out
  out="$(gencap log --quick "x" 2>&1 || true)"
  rm -rf "$SANDBOX"

  # Either the dispatcher's bootstrap_hint OR the log script's slug-missing message.
  assert_contains "bootstrap hint mentions bootstrap-friction" "$out" "bootstrap-friction.sh" || return 1
  return 0
}

# --- TEST 5: kill switch via env var no-ops ---
test_kill_switch_via_env_var() {
  setup_sandbox
  local logf="$HOME/.gstack/projects/jan347-vibe-kanban/friction-log.jsonl"
  : > "$logf"
  GENCAP_FRICTION_ENABLED=0 gencap log --quick "should not appear" >/dev/null 2>&1 || true
  local lines
  lines="$(wc -l < "$logf" | tr -d ' ')"
  teardown_sandbox

  if [[ "$lines" != "0" ]]; then
    echo "  FAIL: kill switch did not no-op; expected 0 lines, got $lines"
    return 1
  fi
  return 0
}

# --- run all tests ---
echo "Running gencap shell tests..."
run_test "test_help_shows_subcommands"           test_help_shows_subcommands
run_test "test_log_quick_appends_line"           test_log_quick_appends_line
run_test "test_log_invalid_venture_shows_template" test_log_invalid_venture_shows_template
run_test "test_log_missing_dir_shows_bootstrap_hint" test_log_missing_dir_shows_bootstrap_hint
run_test "test_kill_switch_via_env_var"          test_kill_switch_via_env_var

echo ""
echo "─────────────────────────"
echo "Results: $PASS passed, $FAIL failed"
if (( FAIL > 0 )); then
  echo "Failed tests:"
  for t in "${FAILED_TESTS[@]}"; do echo "  - $t"; done
  exit 1
fi
exit 0
