#!/usr/bin/env python3
"""Day-3 friction-log summarizer.

Reads events.jsonl + friction-log.jsonl from
~/.gstack/projects/jan347-vibe-kanban/, applies the codified 12-branch
decision tree (docs/designs/friction-log-discipline.md D4 + E-AUTO-3),
and writes Phase 14 CANDIDATES output to docs/designs/phase-14-candidates.md.

Two modes:
  python scripts/reread.py            # writes candidates file + prints summary
  python scripts/reread.py --dry-run  # prints summary, no file write
  python scripts/reread.py --skip-bad # tolerate malformed JSONL with WARN

The 12-branch tree codifies what should happen at day-3 reread for every
plausible state of the data — empty log, low-load, divergence, panel-rederivation,
cluster-on-one-venture, instrumentation failure, etc. Order matters: codex
E-AUTO-3 caught that the original tree had B1 (volume gate) firing BEFORE
B6/B7/B8 manual-wins logic, suppressing the most important signals on
low-event days. This implementation reorders so manual signals get evaluated
first.

Stdlib only. Python 3.9+.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Iterator


# -----------------------------------------------------------------------------
# Constants
# -----------------------------------------------------------------------------

PROJECT_DIR = Path.home() / ".gstack" / "projects" / "jan347-vibe-kanban"
EVENTS_PATH = PROJECT_DIR / "events.jsonl"
FRICTION_LOG_PATH = PROJECT_DIR / "friction-log.jsonl"
EXPERIMENT_START_PATH = PROJECT_DIR / ".experiment-start"
CANDIDATES_OUTPUT_PATH = (
    Path(__file__).resolve().parent.parent
    / "docs"
    / "designs"
    / "phase-14-candidates.md"
)

# Severity weights from E-AUTO-3 (codex fix). Manual P1 must dominate
# noise events: 30 > 20 raw events. Original 2:1 multiplier didn't make
# manual dominate (P1=3*2=6 < 20 noise events).
SEVERITY_WEIGHT = {"P1": 30.0, "P2": 10.0, "P3": 1.0}

# Volume thresholds for B1 (extend dogfood) — codified D4.
MIN_HIGH_SEVERITY_ENTRIES = 5  # < 5 P1/P2 → not enough manual signal
MIN_EVENTS = 30  # < 30 events → not enough auto signal
CLUSTER_THRESHOLD = 0.80  # B12: ≥80% from one venture = unrepresentative

# Schema version readable. v=1 for our writer; readers accept any v <= 1
# until v=2 ships (per friction-log-schema.md versioning policy).
SCHEMA_MAX_VERSION = 1


# -----------------------------------------------------------------------------
# Branches (12 codified outcomes from D4 + E-AUTO-3 + codex T5 fixes)
# -----------------------------------------------------------------------------


class Branch(Enum):
    B1_EXTEND_NO_DATA = "B1: Total entries < 5 P1/P2 OR events < 30 → extend 2 days"
    B2_EXTEND_P3_ONLY = "B2: All entries P3 → extend (P3 = noise)"
    B3_DIVERGENCE = "B3: Manual ↔ events divergence → the misalignment IS the candidate"
    B4_RE_DERIVES_PANEL = "B4: Log re-derives panel list → discount, look at outliers"
    B5_INVESTIGATE_NO_MANUAL = (
        "B5: ≥30 events but no manual frictions → investigate event distribution"
    )
    B6_LOW_LOAD = "B6: Few events from low-load days → distinguish from 'no friction'"
    B7_MANUAL_WINS = "B7: High manual friction, low event support → manual wins (T6)"
    B8_TRUST_MANUAL = "B8: Severe P1 without event support → trust manual signal"
    B9_SUPERVISOR_NOISE = "B9: Supervisor events with zero user friction → P3 / working"
    B10_DREAM_STATE = "B10: Top-2 P1s conflict architecturally → pick dream-state"
    B11_INSTRUMENTATION_FAILURE = (
        "B11: Instrumentation failure → fix and rerun with 1-2 day extension"
    )
    B12_CLUSTER_ONE_VENTURE = (
        "B12: ≥80% entries from single venture → unrepresentative; rotate ventures"
    )
    DEFAULT_PHASE_14_CANDIDATES = "DEFAULT: Top-2 P1 with event support → CANDIDATES"


# -----------------------------------------------------------------------------
# Data classes
# -----------------------------------------------------------------------------


@dataclass
class Entry:
    """One row from friction-log.jsonl."""

    ts: str
    venture: str | None
    layer: str
    severity: str  # P1 | P2 | P3
    what_i_tried: str
    what_blocked: str
    workaround: str | None
    time_lost_min: int

    @classmethod
    def parse(cls, raw: dict) -> Entry | None:
        # Defensive: any field missing or wrong-shape → drop the entry.
        # Skip-on-error in the loader handles per-line parse errors;
        # this is the "valid JSON, wrong shape" guard.
        try:
            sev = str(raw.get("severity", ""))
            if sev not in {"P1", "P2", "P3"}:
                return None
            return cls(
                ts=str(raw["ts"]),
                venture=raw.get("venture"),
                layer=str(raw.get("layer", "external")),
                severity=sev,
                what_i_tried=str(raw.get("what_i_tried", "")),
                what_blocked=str(raw.get("what_blocked", "")),
                workaround=raw.get("workaround") or None,
                time_lost_min=int(raw.get("time_lost_min", 0) or 0),
            )
        except (KeyError, TypeError, ValueError):
            return None


@dataclass
class Event:
    """One row from events.jsonl (v:1 schema)."""

    v: int
    ts: str
    event: str
    venture: str | None
    workspace_id: str | None

    @classmethod
    def parse(cls, raw: dict) -> Event | None:
        try:
            v = int(raw.get("v", 0))
            if v > SCHEMA_MAX_VERSION:
                # v:N+1 reader-forward compat: skip newer-version events
                # that we can't interpret. Forwards-compat is the policy.
                return None
            return cls(
                v=v,
                ts=str(raw["ts"]),
                event=str(raw["event"]),
                venture=raw.get("venture"),
                workspace_id=raw.get("workspace_id"),
            )
        except (KeyError, TypeError, ValueError):
            return None


@dataclass
class ParseStats:
    total_lines: int = 0
    parsed_ok: int = 0
    skipped_bad: int = 0


# -----------------------------------------------------------------------------
# Loaders
# -----------------------------------------------------------------------------


def read_jsonl(path: Path, skip_bad: bool = False) -> tuple[list[dict], ParseStats]:
    """Read a JSONL file with skip-on-error.

    Empty file or missing file → empty list (NOT an error). Malformed
    lines log to stderr with line number + reason. Skipping never raises.
    """
    stats = ParseStats()
    rows: list[dict] = []

    if not path.exists():
        return rows, stats

    try:
        with path.open("r", encoding="utf-8") as f:
            for lineno, raw_line in enumerate(f, start=1):
                stats.total_lines += 1
                line = raw_line.strip()
                if not line:
                    continue
                try:
                    rows.append(json.loads(line))
                    stats.parsed_ok += 1
                except json.JSONDecodeError as e:
                    stats.skipped_bad += 1
                    msg = (
                        f"error: cannot parse {path.name}:{lineno}\n"
                        f"because: {e.msg} at column {e.colno}\n"
                        f"try: edit the line, or rerun with --skip-bad to continue past it"
                    )
                    if skip_bad:
                        print(f"WARN: skipping {path.name}:{lineno} ({e.msg})", file=sys.stderr)
                    else:
                        # Without --skip-bad we still continue — the script
                        # is best-effort by design. But we surface the
                        # problem prominently per DX-AUTO-3.
                        print(msg, file=sys.stderr)
    except OSError as e:
        # Path exists check above is racy; if we get here, log and return what we have.
        print(f"error: cannot read {path}\nbecause: {e}\ntry: check permissions", file=sys.stderr)

    return rows, stats


def load_entries(skip_bad: bool = False) -> tuple[list[Entry], ParseStats]:
    rows, stats = read_jsonl(FRICTION_LOG_PATH, skip_bad)
    parsed = [e for e in (Entry.parse(r) for r in rows) if e is not None]
    return parsed, stats


def load_events(skip_bad: bool = False) -> tuple[list[Event], ParseStats]:
    rows, stats = read_jsonl(EVENTS_PATH, skip_bad)
    parsed = [e for e in (Event.parse(r) for r in rows) if e is not None]
    return parsed, stats


def read_experiment_start() -> datetime | None:
    """Read .experiment-start. Format: YYYY-MM-DD on first line."""
    if not EXPERIMENT_START_PATH.exists():
        return None
    try:
        text = EXPERIMENT_START_PATH.read_text(encoding="utf-8").strip()
        return datetime.strptime(text, "%Y-%m-%d").replace(tzinfo=timezone.utc)
    except (OSError, ValueError):
        return None


# -----------------------------------------------------------------------------
# Aggregations
# -----------------------------------------------------------------------------


def weighted_score(entry: Entry) -> float:
    """E-AUTO-3 codex-fixed formula: P1=30, P2=10, P3=1.

    A single manual P1 (score 30) beats 20 noise events (score 20). Original
    2:1 was insufficient — the math didn't actually make manual dominate.
    """
    return SEVERITY_WEIGHT.get(entry.severity, 0.0)


def venture_event_score(events_for_venture: list[Event]) -> float:
    return float(len(events_for_venture))


def total_weighted_score(entries: list[Entry], events: list[Event]) -> float:
    return sum(weighted_score(e) for e in entries) + venture_event_score(events)


def group_by_venture(entries: list[Entry]) -> dict[str | None, list[Entry]]:
    grouped: dict[str | None, list[Entry]] = {}
    for e in entries:
        grouped.setdefault(e.venture, []).append(e)
    return grouped


def group_events_by_venture(events: list[Event]) -> dict[str | None, list[Event]]:
    grouped: dict[str | None, list[Event]] = {}
    for ev in events:
        grouped.setdefault(ev.venture, []).append(ev)
    return grouped


# -----------------------------------------------------------------------------
# Branch detection
# -----------------------------------------------------------------------------


def is_all_p3(entries: list[Entry]) -> bool:
    """B2 guard. Empty list returns False (not "all P3"); see codex T5 empty-list bug."""
    return len(entries) > 0 and all(e.severity == "P3" for e in entries)


def cluster_ratio_one_venture(entries: list[Entry]) -> float:
    """B12 detector: fraction of entries from the most-represented venture."""
    if not entries:
        return 0.0
    counts: dict[str | None, int] = {}
    for e in entries:
        counts[e.venture] = counts.get(e.venture, 0) + 1
    return max(counts.values()) / len(entries)


# Heuristics for "supervisor.* events but zero user friction" (B9) and
# "instrumentation failure" (B11). These are pattern-based, not exact.

SUPERVISOR_EVENT_PREFIXES = ("supervisor.evaluate.", "supervisor.resolve.")


def supervisor_dominant_no_manual(entries: list[Entry], events: list[Event]) -> bool:
    """B9: supervisor produces >50% of events but the user logged no
    cockpit-coord or agent friction → supervisor working as intended,
    not a friction signal."""
    if not events:
        return False
    sup_count = sum(
        1 for e in events if any(e.event.startswith(p) for p in SUPERVISOR_EVENT_PREFIXES)
    )
    if sup_count / len(events) < 0.50:
        return False
    cockpit_or_agent_friction = any(
        e.layer in {"cockpit-coord", "agent"} for e in entries
    )
    return not cockpit_or_agent_friction


def looks_like_instrumentation_failure(events: list[Event], stats: ParseStats) -> bool:
    """B11: events.jsonl silently broken — > 25% of lines failed to parse,
    OR file exists with zero events but workspaces were created (we don't
    cross-check the DB here, so use the malformed-rate proxy)."""
    if stats.total_lines == 0:
        return False  # missing file is not an instrumentation failure, it's "didn't run"
    if stats.parsed_ok == 0:
        return True
    return stats.skipped_bad / stats.total_lines > 0.25


def looks_like_low_load(events: list[Event], days: int) -> bool:
    """B6: < 30 events because the operator had a slow week. Heuristic:
    < 30 events AND avg events/day < 5. Distinguishes from B5 ("user
    avoided cockpit"); B6 is "user tried, didn't have much to do."
    """
    if days <= 0:
        return False
    return len(events) < MIN_EVENTS and (len(events) / days) < 5.0


def panel_phase_14_keywords() -> list[str]:
    """Keywords that suggest the log is re-deriving the panel's Phase 14
    proposal. Codex tone: low precision is fine — false positives cost
    nothing (user just sees the warning), false negatives cost a wrong
    Phase 14 mint."""
    return [
        "cmd+k",
        "cmdk",
        "quick launch",
        "flash dispatch",
        "ranked inbox",
        "campaign chain",
        "diff control",
        "run notebook",
    ]


def re_derives_panel(entries: list[Entry]) -> bool:
    """B4 detector: ≥50% of P1/P2 entries mention panel keywords."""
    high = [e for e in entries if e.severity in ("P1", "P2")]
    if not high:
        return False
    keywords = panel_phase_14_keywords()
    matched = 0
    for entry in high:
        haystack = (entry.what_i_tried + " " + entry.what_blocked).lower()
        if any(k in haystack for k in keywords):
            matched += 1
    return matched / len(high) >= 0.50


def divergence(entries: list[Entry], events: list[Event]) -> bool:
    """B3 detector: high manual + low events OR low manual + high events.

    The plan's B3 says "log says X, events say Y" — we read this as
    venture-level disagreement: top-friction venture by manual entries
    is NOT in the top-2 ventures by events.
    """
    if not entries or not events:
        return False
    by_v_manual = group_by_venture(entries)
    by_v_events = group_events_by_venture(events)
    top_manual = max(
        by_v_manual,
        key=lambda v: sum(weighted_score(e) for e in by_v_manual[v]),
        default=None,
    )
    if top_manual is None:
        return False
    by_event_count = sorted(by_v_events.items(), key=lambda kv: -len(kv[1]))
    top_2_event_ventures = {v for v, _ in by_event_count[:2]}
    return top_manual not in top_2_event_ventures


def architecturally_conflict(top_2_p1: list[Entry]) -> bool:
    """B10: top-2 P1s land in different layers (e.g., cockpit-ui vs
    cockpit-coord) — picking one would foreclose the other. Heuristic.
    """
    if len(top_2_p1) < 2:
        return False
    return top_2_p1[0].layer != top_2_p1[1].layer


def severe_p1_no_event_support(entries: list[Entry], events: list[Event]) -> bool:
    """B8: a P1 entry exists but no events are venture-tagged for the
    same venture as that P1 — the manual signal stands alone."""
    p1s = [e for e in entries if e.severity == "P1"]
    if not p1s:
        return False
    event_ventures = {ev.venture for ev in events if ev.venture}
    return any(p.venture and p.venture not in event_ventures for p in p1s)


def manual_wins_unsupported_high_severity(
    high_sev: list[Entry], events: list[Event]
) -> bool:
    """B7 detector: ≥1 P1/P2 entry weighted-score-dominates the corresponding
    venture's event count.

    With weighting (P1=30, P2=10), a single P1 = 30 > 20 events. So
    "manual wins" fires when the venture has any P1 OR ≥3 P2s while the
    same venture has < 20 events.
    """
    by_v_events = group_events_by_venture(events)
    for entry in high_sev:
        ventures_events = by_v_events.get(entry.venture, [])
        manual_score = weighted_score(entry)
        event_score = float(len(ventures_events))
        if manual_score > event_score and entry.severity == "P1":
            return True
        # P2 needs to dominate too (single P2 = 10 > 0 events fires; tighten
        # to "P2 with no events at all in that venture")
        if entry.severity == "P2" and not ventures_events:
            return True
    return False


# -----------------------------------------------------------------------------
# Decision tree
# -----------------------------------------------------------------------------


def decide(
    entries: list[Entry],
    events: list[Event],
    events_stats: ParseStats,
    days_elapsed: int,
) -> Branch:
    """The 12-branch tree. ORDER MATTERS — codex E-AUTO-3 fix."""

    # B1 (empty everything) — earliest guard, before any "all" check
    if not entries and not events:
        return Branch.B1_EXTEND_NO_DATA

    # B11 instrumentation failure — silent breakage in events.jsonl
    # checks BEFORE B5 because a high parse-failure rate in events would
    # fake an "events but no manual" signal.
    if looks_like_instrumentation_failure(events, events_stats):
        return Branch.B11_INSTRUMENTATION_FAILURE

    # B5 (manual=0 events>0) — user dispatched but didn't write anything down
    if not entries and events:
        return Branch.B5_INVESTIGATE_NO_MANUAL

    # B2 (all P3) — empty-list guard handled inside is_all_p3
    if is_all_p3(entries):
        return Branch.B2_EXTEND_P3_ONLY

    high_sev = [e for e in entries if e.severity in ("P1", "P2")]

    # B7 (codex T6 asymmetry) — manual signals win regardless of event volume
    if manual_wins_unsupported_high_severity(high_sev, events):
        return Branch.B7_MANUAL_WINS

    # B8 — severe P1 without event support
    if severe_p1_no_event_support(entries, events):
        return Branch.B8_TRUST_MANUAL

    # B9 — supervisor dominant, no user-visible friction
    if supervisor_dominant_no_manual(entries, events):
        return Branch.B9_SUPERVISOR_NOISE

    # Volume gate — distinguish low-load (B6) from genuine "extend" (B1)
    if len(high_sev) < MIN_HIGH_SEVERITY_ENTRIES or len(events) < MIN_EVENTS:
        if looks_like_low_load(events, days_elapsed):
            return Branch.B6_LOW_LOAD
        return Branch.B1_EXTEND_NO_DATA

    # B12 — clustering check
    if cluster_ratio_one_venture(entries) >= CLUSTER_THRESHOLD:
        return Branch.B12_CLUSTER_ONE_VENTURE

    # B4 — re-derives panel
    if re_derives_panel(entries):
        return Branch.B4_RE_DERIVES_PANEL

    # B3 — divergence
    if divergence(entries, events):
        return Branch.B3_DIVERGENCE

    # B10 — top-2 P1s architecturally conflict
    p1_ranked = sorted(
        [e for e in entries if e.severity == "P1"],
        key=lambda e: -weighted_score(e),
    )
    if architecturally_conflict(p1_ranked[:2]):
        return Branch.B10_DREAM_STATE

    return Branch.DEFAULT_PHASE_14_CANDIDATES


# -----------------------------------------------------------------------------
# Phase 14 CANDIDATES output
# -----------------------------------------------------------------------------


def top_candidates(entries: list[Entry], events: list[Event]) -> list[dict]:
    """Top-2 P1 with event support, plus their candidate metadata."""
    by_v_events = group_events_by_venture(events)
    p1s = [e for e in entries if e.severity == "P1"]
    p1_ranked = sorted(p1s, key=lambda e: -weighted_score(e))

    candidates = []
    for entry in p1_ranked[:2]:
        ev_count = len(by_v_events.get(entry.venture, []))
        candidates.append(
            {
                "what_i_tried": entry.what_i_tried,
                "what_blocked": entry.what_blocked,
                "venture": entry.venture or "(none)",
                "layer": entry.layer,
                "events_supporting": ev_count,
                "weighted_score": weighted_score(entry) + ev_count,
            }
        )
    return candidates


def write_candidates_file(branch: Branch, entries: list[Entry], events: list[Event]) -> None:
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    cycle_2_target = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    candidates = top_candidates(entries, events)

    lines: list[str] = []
    lines.append(f"# Phase 14 Candidates (Day-3 reread, {today})")
    lines.append("")
    lines.append(f"**Decision tree branch:** {branch.value}")
    lines.append("")
    lines.append(f"**Total manual entries:** {len(entries)}")
    lines.append(f"**Total events:** {len(events)}")
    lines.append("")

    if branch != Branch.DEFAULT_PHASE_14_CANDIDATES:
        lines.append("## ⚠ Non-default branch — do NOT mint Phase 14 from this run")
        lines.append("")
        lines.append(
            "The decision tree fired a non-default branch. Resolve that "
            "first (extend dogfood, fix instrumentation, rotate ventures, "
            "discount panel-derivation) before treating the candidates "
            "below as actionable."
        )
        lines.append("")

    if not candidates:
        lines.append("## No candidates surfaced")
        lines.append("")
        lines.append(
            "The reread did not find P1 entries that pass the volume + "
            "event-support gates. Extend the dogfood window or adjust the "
            "instrumentation, then rerun."
        )
    else:
        for i, cand in enumerate(candidates, start=1):
            lines.append(f"## Candidate {i}: {cand['what_i_tried']}")
            lines.append("")
            lines.append(f"- **Venture:** {cand['venture']}")
            lines.append(f"- **Layer:** {cand['layer']}")
            lines.append(f"- **What blocked:** {cand['what_blocked']}")
            lines.append(f"- **Events supporting:** {cand['events_supporting']}")
            lines.append(f"- **Weighted score:** {cand['weighted_score']:.1f}")
            lines.append("")

    lines.append("## Cycle-2 validation plan")
    lines.append("")
    lines.append(
        f"Run a second 1-2 week dogfood cycle starting {cycle_2_target}. "
        f"Candidates above become Phase 14 only if they reappear in the "
        f"cycle-2 friction-log AND events. Cycle-2 misses → drop the candidate."
    )
    lines.append("")
    lines.append("## Notes")
    lines.append("")
    lines.append("This file is auto-generated by `scripts/reread.py`. ")
    lines.append("It captures the day-3 candidate output, not a Phase 14 commitment. ")
    lines.append(
        "See `docs/designs/friction-log-discipline.md` for the full discipline "
        "and the 12-branch tree."
    )

    CANDIDATES_OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    CANDIDATES_OUTPUT_PATH.write_text("\n".join(lines) + "\n", encoding="utf-8")


# -----------------------------------------------------------------------------
# Summary printing
# -----------------------------------------------------------------------------


def print_summary(
    branch: Branch,
    entries: list[Entry],
    events: list[Event],
    entry_stats: ParseStats,
    event_stats: ParseStats,
) -> None:
    print("=" * 72)
    print(f"FRICTION LOG REREAD — {datetime.now(timezone.utc).strftime('%Y-%m-%d')}")
    print("=" * 72)
    print()
    print(f"Branch: {branch.value}")
    print()

    print("Manual log:")
    p1 = sum(1 for e in entries if e.severity == "P1")
    p2 = sum(1 for e in entries if e.severity == "P2")
    p3 = sum(1 for e in entries if e.severity == "P3")
    print(
        f"  total: {len(entries)}  |  P1: {p1}  P2: {p2}  P3: {p3}  "
        f"(parsed {entry_stats.parsed_ok}, skipped {entry_stats.skipped_bad})"
    )
    print()

    print("Events:")
    print(
        f"  total: {len(events)}  "
        f"(parsed {event_stats.parsed_ok}, skipped {event_stats.skipped_bad})"
    )
    print()

    print("Per-venture (sorted by weighted_score):")
    by_v = group_by_venture(entries)
    by_v_e = group_events_by_venture(events)
    venture_keys = set(by_v.keys()) | set(by_v_e.keys())
    rows = []
    for v in venture_keys:
        ents = by_v.get(v, [])
        evs = by_v_e.get(v, [])
        score = total_weighted_score(ents, evs)
        rows.append((v or "(none)", len(ents), len(evs), score))
    rows.sort(key=lambda r: -r[3])
    for venture, n_ent, n_ev, score in rows:
        print(f"  {venture:<20} entries={n_ent:<3} events={n_ev:<4} score={score:.1f}")
    print()

    if branch == Branch.DEFAULT_PHASE_14_CANDIDATES:
        candidates = top_candidates(entries, events)
        print(f"Phase 14 CANDIDATES (top {len(candidates)}, validate in cycle-2):")
        for i, cand in enumerate(candidates, start=1):
            print(
                f"  {i}. {cand['what_i_tried']!r} "
                f"(venture: {cand['venture']}, layer: {cand['layer']}, "
                f"events: {cand['events_supporting']})"
            )
    else:
        print(f"Action: {branch.name} — see docs/designs/friction-log-discipline.md D4")
    print()


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Day-3 friction-log summarizer for the GenCap Control Room experiment."
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print summary only; do not write phase-14-candidates.md",
    )
    parser.add_argument(
        "--skip-bad",
        action="store_true",
        help="Tolerate malformed JSONL lines silently (still logs WARN to stderr)",
    )
    args = parser.parse_args(argv)

    entries, entry_stats = load_entries(skip_bad=args.skip_bad)
    events, event_stats = load_events(skip_bad=args.skip_bad)

    start = read_experiment_start()
    if start is None:
        days_elapsed = 0
    else:
        days_elapsed = max(1, (datetime.now(timezone.utc) - start).days + 1)

    branch = decide(entries, events, event_stats, days_elapsed)
    print_summary(branch, entries, events, entry_stats, event_stats)

    if not args.dry_run:
        write_candidates_file(branch, entries, events)
        print(f"✓ Wrote {CANDIDATES_OUTPUT_PATH}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
