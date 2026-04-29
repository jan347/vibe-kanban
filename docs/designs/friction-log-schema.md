# Friction Log Schema

Schema reference for the 3-day friction-log discipline experiment (see [`friction-log-discipline.md`](./friction-log-discipline.md)).

Two stores, one versioning policy:

1. **events store** — automatic, emitted from Rust chokepoints (`crates/services/src/services/friction_emitter.rs`).
2. **friction-log.jsonl** — manual, structured entries written by `gencap log`.

The day-3 summarizer ([`scripts/reread.py`](../../scripts/reread.py)) reads BOTH stores and writes [`phase-14-candidates.md`](./phase-14-candidates.md).

---

## events.jsonl v:1 schema (locked)

Default storage path:
```
~/.gstack/projects/jan347-vibe-kanban/events.jsonl
```

(SQLite alternative path documented under "Versioning Policy" → "SQLite alternative path" below — selected per gate decision E-AUTO-2.)

### Wire format

One JSON object per line. Fields are written in this order by the Rust emitter (deterministic for diff-friendliness; readers MUST NOT rely on ordering).

```jsonl
{"v": 1, "ts": "2026-04-29T17:23:01Z", "event": "snake_case_string", "venture": "string|null", "workspace_id": "32-char-hex|null", "metadata": {}}
```

### Field documentation

| Field | Type | Nullable | Notes |
|-------|------|----------|-------|
| `v` | integer | no | Schema version. Locked at `1` for the experiment. Readers MUST validate `v ≤ supported_max` and skip unknown majors with a stderr warning. |
| `ts` | string (RFC 3339 / ISO 8601, UTC, with `Z` suffix) | no | `chrono::Utc::now().to_rfc3339()`. Always UTC. |
| `event` | string (snake_case) | no | One of the allowed values listed below. Unknown values MUST NOT crash readers — they should be counted as "unrecognized" and surfaced in the day-3 summary. |
| `venture` | string \| null | yes | Read from `workspaces.venture` at emit time. Null when no workspace context exists OR when the workspace row has `venture = NULL`. Allowed string values: `chief-of-staff`, `carbonv3`, `fultech`, `port-analytics`, `other`. |
| `workspace_id` | string (32-char lowercase hex) \| null | yes | `hex::encode(workspace.id)` — 16-byte SQLite BLOB UUID encoded to 32 lowercase hex chars. Null only when no workspace context exists (e.g., supervisor-level events not tied to a workspace). |
| `metadata` | object | no | Free-form JSON object for event-specific context. MUST be `{}` (empty object) when no metadata. NEVER `null`. Schema is event-specific and additive — see "Versioning Policy" below for evolution rules. |

### Allowed `event` values (8 distinct events)

The chokepoint inventory (per E-AUTO-1) emits these events. Each is fired exactly once per logical occurrence, dedup-keyed by the listed primary key.

| Event | Chokepoint | Dedup key (primary key) |
|-------|-----------|--------------------------|
| `workspace.create` | `crates/db/src/models/workspace.rs::create` AFTER commit | `workspaces.id` (one event per persisted row, even if upper handler retries) |
| `dispatch.fire` | `crates/services/src/services/dispatch_guard.rs::gated_create_dispatch` AFTER commit | `dispatch_log.id` |
| `supervisor.evaluate.grant` | `auto_approval.rs::evaluate_and_log` AFTER `tx.commit()`, decision = "approved" | `auto_approval.id` (one per machine decision; not per check) |
| `supervisor.evaluate.deny` | same site, decision = "denied" | `auto_approval.id` |
| `supervisor.resolve.approve` | `routes/safety.rs::resolve_auto_approval` AFTER 2xx response | `auto_approval.id` (one per human resolution) |
| `supervisor.resolve.reject` | same site, opposite resolution | `auto_approval.id` |
| `mail.send` | `crates/db/src/models/mail.rs` mail_messages insert AFTER commit | `mail.id` (one per message_id; fan-out captured in `metadata.recipient_count`) |
| `automation.fire` | `crates/services/src/services/automation_runner.rs` AFTER `last_fired_at` update commit | execution row id |

#### Deprecated events

| Event | Status | Superseded by |
|-------|--------|----------------|
| `auto_approval.grant` | DEPRECATED | `supervisor.evaluate.grant` + `supervisor.resolve.approve` (split — machine decision and human resolution are different signals per E-AUTO-1) |
| `auto_approval.deny` | DEPRECATED | `supervisor.evaluate.deny` + `supervisor.resolve.reject` |

Readers SHOULD accept the deprecated values during the deprecation window (see "Versioning Policy") but treat them as the closest non-deprecated equivalent for aggregation.

### Concurrency model

Per-write strategy in `friction_emitter::emit`:

1. Pre-serialize the full event line (JSON + `\n`) into an in-memory `Vec<u8>` buffer.
2. `OpenOptions::new().create(true).append(true).open(path)` to acquire a file handle.
3. Acquire `flock(LOCK_EX)` advisory lock on the handle.
4. Single `write_all(&buffer)` of the pre-serialized buffer.
5. Drop the handle (releases the lock).

**Why flock + single write_all:** `O_APPEND` alone protects file offset (no two appends overlap in offset) but does NOT guarantee record integrity if a single Rust `write_all` call gets split across syscalls under tokio. PIPE_BUF guarantees apply to pipes, not regular files. Advisory lock + single-syscall write closes both gaps. (See E-AUTO-2 for the full rationale.)

**Writer isolation between files:** events.jsonl is written ONLY by the Rust `friction_emitter` (the server / Tauri app). `friction-log.jsonl` is written ONLY by the `gencap log` shell helper. The two files have separate flock targets and never compete for the same lock — cross-process contention only matters within a file, and each file has exactly one writer process. The empirical Darwin/APFS test (`concurrent_emits_no_interleave`) covers in-process concurrency for events.jsonl, which is the only file that has multiple potential writers (multiple tokio tasks within the server).

**Failure mode:** Swallows `io::Error` to stderr via `tracing::error!`, never panics. The cockpit MUST NOT crash because the event log can't be written.

**Kill switch:** Top of `emit` checks `GENCAP_FRICTION_ENABLED`; if `"0"`, returns immediately (no-op, no rebuild required). See DX-AUTO-7.

---

## friction-log.jsonl schema (manual entry)

Default storage path:
```
~/.gstack/projects/jan347-vibe-kanban/friction-log.jsonl
```

Written exclusively by the `gencap log` CLI helper (per D-AUTO-3 — hand-edit JSONL is forbidden because of escaping foot-guns).

### Wire format

One JSON object per line:

```jsonl
{"ts": "2026-04-29T17:23:01Z", "venture": "chief-of-staff", "layer": "cockpit-ui", "severity": "P2", "what_i_tried": "...", "what_blocked": "...", "workaround": "...", "time_lost_min": 7}
```

### Field documentation

| Field | Type | Nullable | Notes |
|-------|------|----------|-------|
| `ts` | string (ISO 8601 UTC, `Z` suffix) | no | Capture time, written by `gencap log`. |
| `venture` | string | no | One of: `chief-of-staff`, `carbonv3`, `fultech`, `port-analytics`, `other`. (Required for manual entries — operator MUST classify.) |
| `layer` | string | no | One of: `cockpit-ui`, `cockpit-coord`, `agent`, `external`. See definitions below. |
| `severity` | string | no | One of: `P1`, `P2`, `P3`. See definitions below. |
| `what_i_tried` | string | no | Free-form. What the operator was attempting. |
| `what_blocked` | string | no | Free-form. The friction encountered. |
| `workaround` | string | no | Free-form. What the operator did to proceed. May be empty string `""` if no workaround was needed (event was log-only). |
| `time_lost_min` | integer (≥ 0) | no | Minutes lost to the friction. `0` is allowed for "annoyance with no measurable cost." |

### Severity definitions

| Severity | Meaning | Examples |
|----------|---------|----------|
| `P1` | Would-rage-quit. The operator was tempted to abandon the cockpit and use raw shell / direct DB / a different tool. | "Couldn't find which workspace owns a stuck dispatch — gave up and queried SQLite by hand." |
| `P2` | Real productivity hit. Operator stayed in the cockpit but lost meaningful time (>5 min) or had to context-switch. | "Had to refresh /friction three times to see new event — wasted 4 min." |
| `P3` | Annoyance. Notice-and-continue. No measurable productivity impact. | "Color contrast on P3 chips is hard to read in dark mode." |

The day-3 summarizer applies severity weighting per E-AUTO-3:

```python
severity_weight = {"P1": 30.0, "P2": 10.0, "P3": 1.0}[entry.severity]
manual_score = severity_weight  # single P1 = 30, beats 20 noise events
```

P3 entries are explicitly de-weighted; an all-P3 day routes to the `EXTEND` branch (B2).

### Layer definitions

| Layer | Meaning |
|-------|---------|
| `cockpit-ui` | UI confusion — wrong control, missing affordance, unreadable state, mode error in the cockpit's React surface. |
| `cockpit-coord` | Coordination friction — supervisors, dispatch chains, automation rules, multi-step workflows. The friction is in how cockpit pieces talk to each other. |
| `agent` | Agent quality — model produced bad code, hallucinated, ignored instructions, looped. The friction is in the agent itself, not the cockpit's mediation. |
| `external` | Something outside the cockpit — git, shell, third-party API, hardware. Logged for completeness; usually low-priority for cockpit Phase 14. |

### `venture` allowed values

Locked enum (matches the workspace-create UI select per `crates/db/src/models/workspace.rs` migration):

- `chief-of-staff`
- `carbonv3`
- `fultech`
- `port-analytics`
- `other`

The "other" branch in the UI reveals a free-text field; that free-text is NOT preserved in the friction-log (the venture remains `"other"`). Operators who need finer venture distinction should encode it in `what_i_tried` or `what_blocked`.

See [`packages/local-web/AGENTS.md`](../../packages/local-web/AGENTS.md) for the venture color tokens (per D-AUTO-8) used in the `/friction` dashboard.

---

## Versioning Policy

Applies to BOTH `events.jsonl` and `friction-log.jsonl`. The `v:` field exists on `events.jsonl` v:1; `friction-log.jsonl` is implicitly v:1 (manual entries; we will add `v:` if/when we ship a breaking change).

### Minor (additive) changes

- New fields can be added without bumping `v:`.
- Readers MUST accept unknown fields and ignore them.
- Writers SHOULD emit new fields only when populated; absent is interpreted as `null` / default.

### Major (breaking) changes

- Field removal or rename requires `v:N+1`.
- Writers MUST write to BOTH `v:N` and `v:N+1` paths during a transition window of ≥ 7 days.
- Readers MUST accept any `v:M ≤ N` (read-forward compatibility — older readers ignore unrecognized newer majors and warn to stderr).

### Read-write version matrix

| Component | v:1 only (current) | v:2 transition | v:2 stable |
|-----------|---------------------|----------------|------------|
| v:1 reader | reads v:1 | reads v:1 | (deprecated, then removed) |
| v:1 writer | writes v:1 | writes v:1 (mirrored by v:2 writer) | (deprecated, then removed) |
| v:2 reader | n/a | reads v:1 AND v:2 | reads v:1 AND v:2 |
| v:2 writer | n/a | writes v:1 AND v:2 | writes v:2 only (after deprecation cliff) |

### Deprecation timeline

- **t = 0:** v:N+1 ships stable. v:N is marked deprecated. Both writers emit both versions.
- **t + 30 days:** v:N writer is removed. v:N+1 reader still accepts v:N for read-back of historical data.
- **t + 90 days:** v:N reader support is removed. Historical v:N data must be migrated or archived before this cliff.

### SQLite alternative path (E-AUTO-2 Option B)

If the gate decision flips events storage from JSONL to SQLite (`friction_events` table in the existing vibe-kanban DB):

- Schema: `id BLOB PK, ts TEXT, event TEXT, venture TEXT NULL, workspace_id BLOB NULL, metadata JSON, v INTEGER NOT NULL DEFAULT 1`
- Versioning uses standard SQLx migrations (the locked-in pattern for this repo).
- **Additive:** `ALTER TABLE friction_events ADD COLUMN <new>` — no `v:` bump needed for column-level additions.
- **Breaking:** New table + view bridging old → new, with the migration deprecating the old table 30 days after the new one stabilizes (mirrors the JSONL timeline above).
- JSONL export at day-3 reread (`scripts/export-events.sh`) — JSONL becomes the read-only export format, not the source of truth.

The `friction-log.jsonl` (manual entries) remains JSONL regardless of the events-store gate decision — it's append-only operator log and there is no contention with server processes.

---

## Cross-references

- **Design doc / spec:** [`friction-log-discipline.md`](./friction-log-discipline.md) — full plan, off-ramp protocol, day-3 decision tree, accepted scope.
- **Operator Quickstart:** [`friction-log-discipline.md` → "Operator Quickstart"](./friction-log-discipline.md#operator-quickstart) — daily commands, kill switch, off-ramp summary.
- **Venture color tokens (D-AUTO-8):** [`packages/local-web/AGENTS.md`](../../packages/local-web/AGENTS.md) — locks indigo/emerald/violet/sky/zinc per venture.
- **Day-3 summarizer:** [`scripts/reread.py`](../../scripts/reread.py) — reads both stores, applies E-AUTO-3 weighted_score formula, walks the 12-branch decision tree, writes `phase-14-candidates.md`.
- **Emitter source:** `crates/services/src/services/friction_emitter.rs` — implements the per-write flock + single write_all pattern documented above.
- **CLAUDE.md banner (DX-AUTO-6):** [`/CLAUDE.md`](../../CLAUDE.md) — self-contained 3-day banner; cold Claude sessions read this first.
