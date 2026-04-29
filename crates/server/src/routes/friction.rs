//! Friction-log discipline snapshot endpoint (D-AUTO-1 / E-AUTO-4).
//!
//! Background:
//! `docs/designs/friction-log-discipline.md` E-AUTO-4 + D-AUTO-1.
//! `docs/designs/friction-log-schema.md` for the on-disk format.
//!
//! Reads the operator's friction-log + emitter events stores from
//! `~/.gstack/projects/jan347-vibe-kanban/`, aggregates per-venture, and
//! returns a normalised `FrictionSnapshot` for the React `/friction`
//! dashboard to render.
//!
//! Behaviour contract:
//!
//! - Skip-on-error: a malformed JSONL line increments `skipped_lines` and
//!   logs a `tracing::warn`, but never aborts the response. The experiment
//!   IS the input — partial corruption is expected and the dashboard
//!   surfaces it via `skipped_lines`.
//! - Empty / missing file: returns an empty result, NOT an error. The
//!   dashboard renders a polite "no entries yet" surface.
//! - No cache for v1: this is a solo-operator tool with file reads of
//!   <100 KB. Hot reread on every poll is fine, and a cache adds a stale
//!   read window that would mask "did my entry actually land?" feedback —
//!   the worst possible failure mode for the experiment. Revisit if/when
//!   the events file grows past 1 MB.

use std::{collections::HashMap, path::PathBuf};

use axum::{Router, extract::State, response::Json as ResponseJson, routing::get};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

/// Friction store paths (hardcoded — solo operator, single repo, no
/// parameterisation needed yet per spec). Resolved at request time so
/// changes to `$HOME` between server start and request are honoured.
const PROJECT_DIR_RELATIVE: &str = ".gstack/projects/jan347-vibe-kanban";
const EVENTS_FILE: &str = "events.jsonl";
const FRICTION_LOG_FILE: &str = "friction-log.jsonl";
const EXPERIMENT_START_FILE: &str = ".experiment-start";

/// Path to the day-3 candidates output (D-AUTO-1 — appears once `reread.py`
/// has run, gates the dashboard's "summarise" CTA visibility).
const DAY_3_CANDIDATES_PATH: &str = "docs/designs/phase-14-candidates.md";

/// Severity weights from E-AUTO-3. P1 = would-rage-quit, P2 = real
/// productivity hit, P3 = annoyance. A single P1 dominates 30 noise
/// events on the venture sort.
const P1_WEIGHT: f64 = 30.0;
const P2_WEIGHT: f64 = 10.0;
const P3_WEIGHT: f64 = 1.0;
const EVENT_WEIGHT: f64 = 1.0;

/// Truncation cap for the latest-entry preview shown on each venture card.
const WHAT_BLOCKED_PREVIEW_CHARS: usize = 80;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct FrictionSnapshot {
    pub header: FrictionHeader,
    pub ventures: Vec<VentureCard>,
    /// `None` = pre-day-3 (candidates not yet generated). `Some(path)` =
    /// the absolute path the operator can open. UI uses presence as a
    /// boolean; path string is informational.
    pub day_3_status: Option<String>,
    /// Count of malformed JSONL lines we skipped while reading. Surfaced
    /// in the dashboard so the operator notices data corruption early
    /// rather than discovering it at day-3 reread.
    pub skipped_lines: u32,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct FrictionHeader {
    /// Day N of 3, computed from `.experiment-start`. `0` when the
    /// marker is missing (pre-experiment state). Clamped to 1..=3
    /// otherwise so a stale marker on day 7 still displays as day 3.
    pub day_n: u32,
    pub p1_count: u32,
    pub p2_count: u32,
    pub p3_count: u32,
    pub event_count: u32,
    pub last_entry_ts: Option<String>,
    pub last_event_ts: Option<String>,
    /// Hours from now until day-3 09:00 UTC cutoff. Negative if past.
    /// `i32` (not `u32`) because the post-day-3 case is real and needs
    /// to be representable.
    pub hours_to_day_3: i32,
    /// ISO 8601 from `.experiment-start`, midnight UTC of the date in
    /// the marker. `None` when the marker is missing.
    pub experiment_start: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct VentureCard {
    pub venture: String,
    pub p1_count: u32,
    pub p2_count: u32,
    pub p3_count: u32,
    pub event_count: u32,
    pub time_lost_min_total: i64,
    /// `P1*30 + P2*10 + P3*1 + events*1.0` — drives the venture sort
    /// (descending). Per E-AUTO-3 weighting.
    pub weighted_score: f64,
    pub latest_entry: Option<LatestEntry>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct LatestEntry {
    pub ts: String,
    pub layer: String,
    pub severity: String,
    /// Truncated to ~80 chars on a UTF-8 char boundary so multi-byte
    /// chars (emoji, accented Czech) don't produce invalid output.
    pub what_blocked: String,
}

// -------- on-disk record shapes (deserialise targets) ------------------------

/// Manual entry shape from `friction-log.jsonl`. All fields required per
/// schema doc — `Deserialize` will reject lines missing any of them and
/// our skip-on-error path will count those as `skipped_lines`.
#[derive(Debug, Deserialize)]
struct LogEntry {
    ts: String,
    venture: String,
    layer: String,
    severity: String,
    #[allow(dead_code)] // reserved for future drill-down view
    what_i_tried: String,
    what_blocked: String,
    #[allow(dead_code)] // reserved for future drill-down view
    workaround: String,
    time_lost_min: i64,
}

/// Auto-emitted event shape from `events.jsonl` (v:1 schema). Only the
/// fields the dashboard needs are deserialised — extra fields are
/// silently ignored per the additive-versioning policy.
#[derive(Debug, Deserialize)]
struct EventEntry {
    ts: String,
    /// Optional per schema. Events without a venture are still counted
    /// in the header total but don't appear on any venture card.
    venture: Option<String>,
}

// -------- handler ------------------------------------------------------------

pub async fn get_friction_snapshot(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<FrictionSnapshot>>, ApiError> {
    let project_dir = match project_dir() {
        Some(dir) => dir,
        None => {
            // No $HOME — extremely rare; surface a 500 with the
            // structured error/because/try shape per DX-AUTO-3.
            tracing::warn!("friction snapshot: $HOME not resolvable");
            return Err(ApiError::BadGateway(
                "error: friction snapshot unavailable. \
                 because: $HOME could not be resolved. \
                 try: confirm the server's HOME environment variable is set."
                    .into(),
            ));
        }
    };

    let mut skipped_lines: u32 = 0;

    let log_entries =
        read_jsonl::<LogEntry>(&project_dir.join(FRICTION_LOG_FILE), &mut skipped_lines).await;
    let events =
        read_jsonl::<EventEntry>(&project_dir.join(EVENTS_FILE), &mut skipped_lines).await;
    let experiment_start =
        read_experiment_start(&project_dir.join(EXPERIMENT_START_FILE)).await;

    let now = Utc::now();
    let header = build_header(&log_entries, &events, experiment_start, now);
    let ventures = build_venture_cards(&log_entries, &events);
    let day_3_status = day_3_status().await;

    Ok(ResponseJson(ApiResponse::success(FrictionSnapshot {
        header,
        ventures,
        day_3_status,
        skipped_lines,
    })))
}

// -------- file IO helpers ----------------------------------------------------

fn project_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(PROJECT_DIR_RELATIVE))
}

/// Reads a JSONL file line-by-line, returning successfully-parsed records.
/// Missing file → empty `Vec` (not an error). Malformed lines → tracing
/// warn + bump `skipped_lines`, continue.
async fn read_jsonl<T>(path: &PathBuf, skipped: &mut u32) -> Vec<T>
where
    T: for<'de> Deserialize<'de>,
{
    let contents = match tokio::fs::read_to_string(path).await {
        Ok(s) => s,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            // Other IO errors (permissions, IO error) are still soft —
            // dashboard should render rather than 500 the user.
            tracing::warn!(
                target: "friction_snapshot",
                path = %path.display(),
                error = %err,
                "could not read JSONL; treating as empty",
            );
            return Vec::new();
        }
    };

    let mut out = Vec::new();
    for (idx, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(line) {
            Ok(record) => out.push(record),
            Err(err) => {
                *skipped = skipped.saturating_add(1);
                tracing::warn!(
                    target: "friction_snapshot",
                    path = %path.display(),
                    line_number = idx + 1,
                    error = %err,
                    "skipped malformed JSONL line",
                );
            }
        }
    }
    out
}

/// Reads `.experiment-start` and returns a UTC midnight `DateTime` for
/// the date written there. Accepts `YYYY-MM-DD` (the canonical form).
/// Missing file or parse failure → `None`, treated as "experiment not
/// started" by the header builder.
async fn read_experiment_start(path: &PathBuf) -> Option<DateTime<Utc>> {
    let contents = tokio::fs::read_to_string(path).await.ok()?;
    let trimmed = contents.trim();
    let date = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok()?;
    Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
        .single()
}

async fn day_3_status() -> Option<String> {
    // Resolved relative to CWD — server is launched from the repo root,
    // matching the rest of the routes (e.g. dev_assets paths). If the
    // operator runs from elsewhere this is `None` rather than wrong.
    let path = PathBuf::from(DAY_3_CANDIDATES_PATH);
    let absolute = tokio::fs::canonicalize(&path).await.ok()?;
    Some(absolute.to_string_lossy().into_owned())
}

// -------- aggregation --------------------------------------------------------

fn build_header(
    log_entries: &[LogEntry],
    events: &[EventEntry],
    experiment_start: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> FrictionHeader {
    let mut p1 = 0u32;
    let mut p2 = 0u32;
    let mut p3 = 0u32;
    let mut last_entry_ts: Option<&str> = None;
    for entry in log_entries {
        match entry.severity.as_str() {
            "P1" => p1 = p1.saturating_add(1),
            "P2" => p2 = p2.saturating_add(1),
            "P3" => p3 = p3.saturating_add(1),
            // Unknown severities are NOT counted in any bucket and NOT
            // skipped — they survived JSON parse so we keep them in
            // `latest_entry` consideration, but they don't contribute
            // to scoring. Dashboard will show the raw severity string.
            _ => {}
        }
        last_entry_ts = max_ts(last_entry_ts, Some(entry.ts.as_str()));
    }

    let mut last_event_ts: Option<&str> = None;
    for event in events {
        last_event_ts = max_ts(last_event_ts, Some(event.ts.as_str()));
    }

    let day_n = compute_day_n(experiment_start, now);
    let hours_to_day_3 = compute_hours_to_day_3(experiment_start, now);

    FrictionHeader {
        day_n,
        p1_count: p1,
        p2_count: p2,
        p3_count: p3,
        event_count: u32::try_from(events.len()).unwrap_or(u32::MAX),
        last_entry_ts: last_entry_ts.map(str::to_string),
        last_event_ts: last_event_ts.map(str::to_string),
        hours_to_day_3,
        experiment_start: experiment_start.map(|dt| dt.to_rfc3339()),
    }
}

fn build_venture_cards(log_entries: &[LogEntry], events: &[EventEntry]) -> Vec<VentureCard> {
    #[derive(Default)]
    struct Acc<'a> {
        p1: u32,
        p2: u32,
        p3: u32,
        events: u32,
        time_lost_min: i64,
        latest: Option<&'a LogEntry>,
    }

    let mut by_venture: HashMap<String, Acc> = HashMap::new();

    for entry in log_entries {
        let acc = by_venture.entry(entry.venture.clone()).or_default();
        match entry.severity.as_str() {
            "P1" => acc.p1 = acc.p1.saturating_add(1),
            "P2" => acc.p2 = acc.p2.saturating_add(1),
            "P3" => acc.p3 = acc.p3.saturating_add(1),
            _ => {}
        }
        // saturating_add for i64 in case of pathological data; clamps
        // at i64::MAX rather than panicking.
        acc.time_lost_min = acc.time_lost_min.saturating_add(entry.time_lost_min);
        // Track the latest entry by ts string compare (RFC 3339 / ISO
        // 8601 with `Z` suffix sorts lexicographically — see schema doc).
        match acc.latest {
            None => acc.latest = Some(entry),
            Some(prev) if entry.ts.as_str() > prev.ts.as_str() => acc.latest = Some(entry),
            _ => {}
        }
    }

    for event in events {
        // Only events with a venture tag land on a venture card. Events
        // without a venture (supervisor-level / global) still land in
        // the header total but not here.
        if let Some(venture) = event.venture.as_ref() {
            let acc = by_venture.entry(venture.clone()).or_default();
            acc.events = acc.events.saturating_add(1);
        }
    }

    let mut cards: Vec<VentureCard> = by_venture
        .into_iter()
        .filter(|(_, acc)| {
            // Hide ventures with zero entries AND zero events — empty
            // cards are visual noise per spec.
            acc.p1 + acc.p2 + acc.p3 + acc.events > 0
        })
        .map(|(venture, acc)| {
            let weighted_score = (acc.p1 as f64) * P1_WEIGHT
                + (acc.p2 as f64) * P2_WEIGHT
                + (acc.p3 as f64) * P3_WEIGHT
                + (acc.events as f64) * EVENT_WEIGHT;

            let latest_entry = acc.latest.map(|entry| LatestEntry {
                ts: entry.ts.clone(),
                layer: entry.layer.clone(),
                severity: entry.severity.clone(),
                what_blocked: truncate_chars(&entry.what_blocked, WHAT_BLOCKED_PREVIEW_CHARS),
            });

            VentureCard {
                venture,
                p1_count: acc.p1,
                p2_count: acc.p2,
                p3_count: acc.p3,
                event_count: acc.events,
                time_lost_min_total: acc.time_lost_min,
                weighted_score,
                latest_entry,
            }
        })
        .collect();

    // Sort by weighted_score descending; tie-break by venture name
    // ascending so the order is deterministic across reloads.
    cards.sort_by(|a, b| {
        b.weighted_score
            .partial_cmp(&a.weighted_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.venture.cmp(&b.venture))
    });
    cards
}

// -------- pure helpers -------------------------------------------------------

fn max_ts<'a>(a: Option<&'a str>, b: Option<&'a str>) -> Option<&'a str> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if x > y { x } else { y }),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

fn compute_day_n(start: Option<DateTime<Utc>>, now: DateTime<Utc>) -> u32 {
    let Some(start) = start else {
        return 0;
    };
    // `today - start` then +1 so the start day itself is "Day 1". Clamp
    // to 1..=3 — past day 3, the dashboard still shows day 3 (the
    // candidates view takes over via day_3_status).
    let days = (now.date_naive() - start.date_naive()).num_days();
    let day_n = days.saturating_add(1);
    day_n.clamp(1, 3) as u32
}

fn compute_hours_to_day_3(start: Option<DateTime<Utc>>, now: DateTime<Utc>) -> i32 {
    let Some(start) = start else {
        return 0;
    };
    // Target = start + 3 days at 09:00 UTC. The 09:00 cutoff matches
    // the operator quickstart's "morning of day 3" reread schedule.
    let target = start + chrono::Duration::days(3) + chrono::Duration::hours(9);
    let delta_hours = (target - now).num_hours();
    i32::try_from(delta_hours).unwrap_or(if delta_hours.is_negative() {
        i32::MIN
    } else {
        i32::MAX
    })
}

/// Truncate to `max_chars` USER-PERCEIVED chars (Rust `chars()` — close
/// enough for the previews we render; no grapheme cluster handling).
/// Adds an ellipsis only if truncation actually occurred.
fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut count = 0usize;
    let mut end_byte = 0usize;
    for (idx, _ch) in input.char_indices() {
        if count == max_chars {
            return format!("{}…", &input[..end_byte]);
        }
        end_byte = idx + input[idx..].chars().next().map(char::len_utf8).unwrap_or(0);
        count += 1;
    }
    input.to_string()
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new().route("/friction/snapshot", get(get_friction_snapshot))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).single().unwrap()
    }

    #[test]
    fn day_n_zero_when_no_start_marker() {
        assert_eq!(compute_day_n(None, ymd(2026, 4, 28)), 0);
    }

    #[test]
    fn day_n_clamps_to_three_when_past() {
        let start = ymd(2026, 4, 1);
        assert_eq!(compute_day_n(Some(start), ymd(2026, 4, 30)), 3);
    }

    #[test]
    fn day_n_starts_at_one_on_start_day() {
        let start = ymd(2026, 4, 28);
        assert_eq!(compute_day_n(Some(start), ymd(2026, 4, 28)), 1);
    }

    #[test]
    fn hours_to_day_3_is_negative_when_past() {
        let start = ymd(2026, 4, 1);
        let now = ymd(2026, 4, 30);
        assert!(compute_hours_to_day_3(Some(start), now) < 0);
    }

    #[test]
    fn truncate_chars_handles_multibyte() {
        // Each emoji is 4 bytes / 1 char — make sure we slice on char
        // boundaries, not byte boundaries.
        let s = "🔥🔥🔥🔥🔥";
        assert_eq!(truncate_chars(s, 3), "🔥🔥🔥…");
        assert_eq!(truncate_chars(s, 10), "🔥🔥🔥🔥🔥");
    }

    #[test]
    fn truncate_chars_no_ellipsis_when_short() {
        assert_eq!(truncate_chars("abc", 80), "abc");
    }

    #[test]
    fn build_venture_cards_hides_empty_ventures() {
        let cards = build_venture_cards(&[], &[]);
        assert!(cards.is_empty());
    }

    #[test]
    fn build_venture_cards_sorts_by_weighted_score_desc() {
        let log_entries = vec![
            LogEntry {
                ts: "2026-04-28T09:00:00Z".into(),
                venture: "fultech".into(),
                layer: "agent".into(),
                severity: "P1".into(),
                what_i_tried: "x".into(),
                what_blocked: "y".into(),
                workaround: "z".into(),
                time_lost_min: 10,
            },
            LogEntry {
                ts: "2026-04-28T10:00:00Z".into(),
                venture: "carbonv3".into(),
                layer: "cockpit-ui".into(),
                severity: "P3".into(),
                what_i_tried: "x".into(),
                what_blocked: "y".into(),
                workaround: "z".into(),
                time_lost_min: 1,
            },
        ];
        let cards = build_venture_cards(&log_entries, &[]);
        assert_eq!(cards.len(), 2);
        // fultech P1 (30) beats carbonv3 P3 (1).
        assert_eq!(cards[0].venture, "fultech");
        assert_eq!(cards[0].weighted_score, 30.0);
        assert_eq!(cards[1].venture, "carbonv3");
        assert_eq!(cards[1].weighted_score, 1.0);
    }

    #[test]
    fn build_header_counts_severities_across_ventures() {
        let log_entries = vec![
            LogEntry {
                ts: "2026-04-28T09:00:00Z".into(),
                venture: "fultech".into(),
                layer: "agent".into(),
                severity: "P1".into(),
                what_i_tried: "x".into(),
                what_blocked: "y".into(),
                workaround: "z".into(),
                time_lost_min: 5,
            },
            LogEntry {
                ts: "2026-04-28T10:00:00Z".into(),
                venture: "carbonv3".into(),
                layer: "cockpit-ui".into(),
                severity: "P2".into(),
                what_i_tried: "x".into(),
                what_blocked: "y".into(),
                workaround: "z".into(),
                time_lost_min: 7,
            },
        ];
        let header = build_header(&log_entries, &[], None, ymd(2026, 4, 28));
        assert_eq!(header.p1_count, 1);
        assert_eq!(header.p2_count, 1);
        assert_eq!(header.p3_count, 0);
        assert_eq!(header.event_count, 0);
        assert_eq!(header.day_n, 0);
    }
}
