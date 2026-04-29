//! Friction-log discipline event emitter (v:1).
//!
//! Background:
//! `docs/designs/friction-log-discipline.md` E-AUTO-1 + E-AUTO-2 + DX-AUTO-7.
//! `docs/designs/friction-log-schema.md` for the v:1 schema and dedup rules.
//!
//! Behavior contract:
//!
//! - Append-only JSONL writes to `~/.gstack/projects/jan347-vibe-kanban/events.jsonl`.
//! - Cross-process safe: a `gencap log` shell process and the Rust server both
//!   write to the same file. We hold an advisory `flock(LOCK_EX)` for the
//!   duration of each write.
//! - Single `write_all` of one pre-serialized buffer (header + JSON + `\n`)
//!   per emit so a partial write inside the lock window can't interleave
//!   with someone who saw stale offset data. Codex's E-AUTO-2 caveat:
//!   `O_APPEND` alone does not protect record integrity across processes.
//! - **Failures NEVER bubble up.** A disk-full or permission-denied error on
//!   the events file MUST NOT crash the cockpit. We log via `tracing::warn`
//!   (debounced) and return `()`.
//! - Kill switch: if `GENCAP_FRICTION_ENABLED=0` is in env, `emit` is a no-op.
//!   No file open, no lock, no write. Lets the user disable instrumentation
//!   without rebuilding (DX-AUTO-7).
//! - Async-friendly: `emit` is `async fn`. Internally uses
//!   `tokio::task::spawn_blocking` for the `flock + write_all` syscalls.
//!   Lock contention is bounded — events are <500 bytes, holding the lock
//!   for ~microseconds.
//!
//! Emit is called AFTER `tx.commit().await?` at the chokepoint. Never inside
//! a transaction. Codex's E-AUTO-1 caveat: emit-inside-tx logs events for
//! transactions that get rolled back, polluting the data.

use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use chrono::Utc;
use fs2::FileExt;
use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Schema version. Bumped only on breaking changes per the read-write
/// version matrix in friction-log-schema.md (additive fields stay v:1).
const SCHEMA_VERSION: u8 = 1;

const EVENTS_DIR_RELATIVE: &str = ".gstack/projects/jan347-vibe-kanban";
const EVENTS_FILE: &str = "events.jsonl";

/// Suppresses repeated stderr spam if the events file is genuinely
/// unwritable (disk full, permission revoked). The first failure logs;
/// subsequent failures stay silent until the next process restart.
static FAILURE_LOGGED: AtomicBool = AtomicBool::new(false);

/// One emitted event. Keep field order stable — this is the schema readers
/// depend on and the version policy is read-forward, not order-sensitive,
/// but tooling (jq, eyeballing) prefers ordered fields.
#[derive(Debug, Serialize)]
struct EventRecord<'a> {
    v: u8,
    ts: String,
    event: &'a str,
    venture: Option<&'a str>,
    workspace_id: Option<String>,
    metadata: serde_json::Value,
}

/// Public emit entry point. See module docs for the contract.
///
/// Async because callers (handlers, services) are async; internally
/// spawns the blocking lock+write on a tokio blocking thread.
pub async fn emit(
    event: &str,
    venture: Option<&str>,
    workspace_id: Option<Uuid>,
    metadata: serde_json::Value,
) {
    if kill_switch_set() {
        return;
    }

    let record = EventRecord {
        v: SCHEMA_VERSION,
        ts: Utc::now().to_rfc3339(),
        event,
        venture,
        workspace_id: workspace_id.map(|id| id.simple().to_string()),
        metadata,
    };

    // serde_json::to_vec failure is essentially impossible for our
    // schema (no NaN, no map<non-string-key>); if it does fail, we log
    // and drop the event rather than panic.
    let mut buf = match serde_json::to_vec(&record) {
        Ok(buf) => buf,
        Err(err) => {
            log_failure_once("serialize", &err.to_string());
            return;
        }
    };
    buf.push(b'\n');

    // Move the prepared buffer onto a blocking thread for the lock+write.
    // The whole sequence (open + flock + write_all + unlock + close)
    // happens there; we don't hold the lock across an await point.
    let _ = tokio::task::spawn_blocking(move || write_locked(buf)).await;
}

fn kill_switch_set() -> bool {
    matches!(std::env::var("GENCAP_FRICTION_ENABLED").as_deref(), Ok("0"))
}

fn write_locked(buf: Vec<u8>) {
    let path = match events_path() {
        Some(path) => path,
        None => {
            log_failure_once("home_dir", "could not resolve $HOME");
            return;
        }
    };

    // Best-effort parent dir creation (no-op if it already exists). This
    // covers the codex H2 finding: the directory may not exist on first
    // emit on a clean machine. Failure here cascades to the open() error
    // below, which already gets debounced-logged.
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(err) => {
            log_failure_once("open", &format!("{}: {}", path.display(), err));
            return;
        }
    };

    // LOCK_EX — exclusive, blocks until acquired. Bounded wait because
    // emits hold the lock for microseconds (one write of <500 bytes).
    if let Err(err) = file.lock_exclusive() {
        log_failure_once("flock", &err.to_string());
        return;
    }

    let mut file = file;
    let write_result = file.write_all(&buf);

    // Always release the lock, even if the write failed. fs2's `unlock`
    // returns Result; we ignore it because nothing useful happens on
    // unlock failure (the OS releases on close anyway).
    let _ = FileExt::unlock(&file);

    if let Err(err) = write_result {
        log_failure_once("write", &err.to_string());
    }
}

fn events_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(EVENTS_DIR_RELATIVE).join(EVENTS_FILE))
}

fn log_failure_once(stage: &str, detail: &str) {
    if FAILURE_LOGGED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        tracing::warn!(
            target: "friction_emitter",
            stage = stage,
            detail = detail,
            "friction event emit failed (subsequent failures suppressed)"
        );
    }
}

// -----------------------------------------------------------------------------
// Convenience helpers — chokepoint call sites use these so the right venture
// is read from the workspace row (E-AUTO-1) without each handler re-implementing
// the lookup.
// -----------------------------------------------------------------------------

/// Emit an event tied to a workspace, looking up the venture tag.
///
/// On lookup failure (workspace deleted between commit and emit, db
/// transient error), the event is still emitted with `venture: null`.
/// Losing the tag is preferable to losing the event entirely.
pub async fn emit_for_workspace(
    pool: &SqlitePool,
    event: &str,
    workspace_id: Uuid,
    metadata: serde_json::Value,
) {
    let venture = lookup_venture(pool, workspace_id).await;
    emit(event, venture.as_deref(), Some(workspace_id), metadata).await;
}

/// Emit an event with no workspace context (e.g. global config touches).
pub async fn emit_global(event: &str, metadata: serde_json::Value) {
    emit(event, None, None, metadata).await;
}

async fn lookup_venture(pool: &SqlitePool, workspace_id: Uuid) -> Option<String> {
    // query!() avoids the Option<Option<String>> type-inference fight
    // that bare query_scalar! has on a nullable column. Anonymous-row
    // access is fine for a one-column lookup.
    sqlx::query!(
        r#"SELECT venture FROM workspaces WHERE id = ?1"#,
        workspace_id
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|row| row.venture)
}

#[cfg(test)]
mod tests {
    use std::{io::Read, sync::Arc};

    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    /// Set up an isolated $HOME for the test so emits write into a tempdir
    /// rather than the user's real ~/.gstack. Returns the expected events
    /// file path.
    fn isolate_home() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("tempdir");
        // SAFETY: tests run sequentially per file by default; we restore
        // via the TempDir drop. The real risk is test parallelism within
        // the same module — these tests use the same env var, so cargo's
        // test threads can race. We rely on cargo running them in a single
        // thread by setting `--test-threads=1`-equivalent. For now,
        // `set_var` is acceptable for the small unit-test surface here.
        unsafe {
            std::env::set_var("HOME", dir.path());
            std::env::remove_var("GENCAP_FRICTION_ENABLED");
        }
        FAILURE_LOGGED.store(false, Ordering::Relaxed);
        let path = dir.path().join(EVENTS_DIR_RELATIVE).join(EVENTS_FILE);
        (dir, path)
    }

    #[tokio::test(flavor = "current_thread")]
    async fn emit_writes_a_v1_jsonl_line() {
        let (_home, path) = isolate_home();
        emit("workspace.create", Some("carbonv3"), None, json!({"hello": true})).await;

        let mut content = String::new();
        std::fs::File::open(&path)
            .expect("file exists")
            .read_to_string(&mut content)
            .expect("readable");
        assert!(content.ends_with('\n'), "must terminate with newline");
        let line = content.trim_end();
        let parsed: serde_json::Value = serde_json::from_str(line).expect("valid json");
        assert_eq!(parsed["v"], 1);
        assert_eq!(parsed["event"], "workspace.create");
        assert_eq!(parsed["venture"], "carbonv3");
        assert_eq!(parsed["metadata"], json!({"hello": true}));
        assert!(parsed["ts"].as_str().unwrap().contains('T'));
        assert!(parsed["workspace_id"].is_null());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn emit_creates_parent_dir_on_first_call() {
        let (_home, path) = isolate_home();
        // Parent dir doesn't exist yet — codex H2 caveat. emit must
        // create_dir_all itself rather than relying on bootstrap.
        assert!(!path.parent().unwrap().exists());
        emit("dispatch.fire", None, None, json!({})).await;
        assert!(path.exists(), "events file created");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn emit_noop_when_kill_switch_set() {
        let (_home, path) = isolate_home();
        unsafe {
            std::env::set_var("GENCAP_FRICTION_ENABLED", "0");
        }
        emit("supervisor.evaluate.deny", None, None, json!({})).await;
        assert!(!path.exists(), "no file when killed");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn emit_includes_workspace_id_as_simple_hex() {
        let (_home, path) = isolate_home();
        let id = Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap();
        emit("mail.send", Some("fultech"), Some(id), json!({})).await;
        let content = std::fs::read_to_string(&path).expect("readable");
        let parsed: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        // Simple form: 32 chars, no hyphens
        assert_eq!(parsed["workspace_id"], "12345678123412341234123456789abc");
    }

    /// Concurrent emits should not produce malformed JSON. With flock + a
    /// single write_all per emit, every line in the resulting file must
    /// parse as JSON and the line count must equal the emit count. This
    /// is the empirical Darwin/APFS atomicity test from E-AUTO-2.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_emits_no_interleave() {
        let (_home, path) = isolate_home();
        let parallelism = 50_usize;
        let mut handles = Vec::with_capacity(parallelism);
        for i in 0..parallelism {
            handles.push(tokio::spawn(async move {
                emit(
                    "automation.fire",
                    Some("carbonv3"),
                    None,
                    json!({"i": i}),
                )
                .await;
            }));
        }
        for h in handles {
            h.await.expect("emit task");
        }

        let content = std::fs::read_to_string(&path).expect("readable");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), parallelism, "no lines lost");
        for line in lines {
            let _: serde_json::Value =
                serde_json::from_str(line).expect("each line is valid JSON");
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn emit_swallows_non_writable_path() {
        // Point HOME at a path with a file (not a dir) where the events
        // dir would go. create_dir_all on top of a regular file fails,
        // emit must swallow it, no panic, no crash.
        let dir = TempDir::new().unwrap();
        let blocker_path = dir.path().join(EVENTS_DIR_RELATIVE);
        std::fs::create_dir_all(blocker_path.parent().unwrap()).unwrap();
        // Make EVENTS_DIR_RELATIVE a *file* — create_dir_all will refuse.
        std::fs::write(&blocker_path, b"not a dir").unwrap();
        unsafe {
            std::env::set_var("HOME", dir.path());
        }
        FAILURE_LOGGED.store(false, Ordering::Relaxed);
        // Should not panic.
        emit("workspace.create", None, None, json!({})).await;
    }

    #[test]
    fn schema_version_constant_matches_doc() {
        // Tripwire: if anyone bumps SCHEMA_VERSION here without updating
        // the docs/designs/friction-log-schema.md versioning policy,
        // future readers will be confused. Keep the assertion explicit.
        assert_eq!(SCHEMA_VERSION, 1);
        let _ = Arc::new(()); // touches Arc import to silence dead_code
    }
}
