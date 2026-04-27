//! Single chokepoint that every dispatch must flow through, regardless
//! of trigger source (UI, automation rule, scheduled runner, MCP tool).
//!
//! Calling [`gated_create_dispatch`] guarantees:
//! 1. The workspace exists (defends against typo'd or stale UUIDs).
//! 2. The auto-approval supervisor was consulted via
//!    [`crate::services::auto_approval::evaluate_and_log`], so an
//!    audit row is written even for blocked attempts.
//! 3. The `dispatch_log` row + `work_item_runs` row are written in a
//!    single transaction (no orphan dispatches on partial failure).
//!
//! If the supervisor returns `denied` or `escalated`, no dispatch row
//! is created and the caller receives `Blocked(decision)` so it can
//! surface the reason to the user without retrying.

use db::models::{
    dispatch::{CreateDispatch, DispatchLogEntry, DispatchStatus},
    safety::AutoApprovalDecision,
};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::services::auto_approval;

#[derive(Debug)]
pub enum GatedDispatchResult {
    Approved(DispatchLogEntry),
    Blocked(AutoApprovalDecision),
}

#[derive(Debug, thiserror::Error)]
pub enum GatedDispatchError {
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(Uuid),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

pub async fn gated_create_dispatch(
    pool: &SqlitePool,
    req: &CreateDispatch,
    action_kind: &str,
    action_summary: &str,
) -> Result<GatedDispatchResult, GatedDispatchError> {
    // Workspace existence guard — without this, a malformed UUID writes
    // an orphan auto_approval_log row and (worse) an orphan dispatch
    // row whose FK to a non-existent workspace silently NULLs.
    let exists: Option<Uuid> = sqlx::query_scalar!(
        r#"SELECT id AS "id!: Uuid" FROM workspaces WHERE id = ?1"#,
        req.workspace_id
    )
    .fetch_optional(pool)
    .await?;
    if exists.is_none() {
        return Err(GatedDispatchError::WorkspaceNotFound(req.workspace_id));
    }

    let decision =
        auto_approval::evaluate_and_log(pool, req.workspace_id, action_kind, action_summary, None)
            .await?;
    if !decision.approved {
        return Ok(GatedDispatchResult::Blocked(decision));
    }

    // Atomic dispatch + work_item_runs insert. If either fails, neither
    // row sticks — the run-counter and dispatch list stay in sync.
    let id = Uuid::new_v4();
    let status = DispatchStatus::Pending;
    let mut tx = pool.begin().await?;
    sqlx::query!(
        r#"INSERT INTO dispatch_log (id, work_item_id, workspace_id, prompt_text,
           prompt_template_id, model_preset_id, status)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
        id,
        req.work_item_id,
        req.workspace_id,
        req.prompt_text,
        req.prompt_template_id,
        req.model_preset_id,
        status
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT OR IGNORE INTO work_item_runs (work_item_id, workspace_id, role) VALUES (?1, ?2, 'dispatch')",
        req.work_item_id,
        req.workspace_id,
    )
    .execute(&mut *tx)
    .await?;
    let row = sqlx::query_as!(
        DispatchLogEntry,
        r#"SELECT id AS "id!: Uuid", work_item_id AS "work_item_id!: Uuid",
           workspace_id AS "workspace_id!: Uuid", session_id AS "session_id?: Uuid",
           prompt_template_id AS "prompt_template_id?: Uuid", prompt_text,
           model_preset_id AS "model_preset_id?: Uuid",
           status AS "status!: DispatchStatus",
           started_at AS "started_at!: chrono::DateTime<chrono::Utc>",
           completed_at AS "completed_at?: chrono::DateTime<chrono::Utc>",
           error_message
           FROM dispatch_log WHERE id = ?1"#,
        id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(GatedDispatchResult::Approved(row))
}
