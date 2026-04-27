use axum::{
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::safety::{
    AutoApprovalDecision, AutoApprovalLogEntry, AutoApprovalRequest, ResolveAutoApprovalRequest,
    SafetyCheckResult, SafetyConfig, UpdateSafetyConfig,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub async fn get_global_config(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<SafetyConfig>>, ApiError> {
    let pool = &deployment.db().pool;
    let row = sqlx::query_as!(
        SafetyConfig,
        r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id?: Uuid",
           scope, require_human_approval AS "require_human_approval!: bool",
           max_concurrent_dispatch AS "max_concurrent_dispatch!: i64",
           max_daily_dispatch AS "max_daily_dispatch!: i64",
           cooldown_seconds AS "cooldown_seconds!: i64",
           auto_approval_enabled AS "auto_approval_enabled!: bool",
           auto_approval_policy,
           auto_approval_model_preset_id AS "auto_approval_model_preset_id?: Uuid",
           created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
           updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
           FROM safety_config WHERE scope = 'global' LIMIT 1"#
    )
    .fetch_one(pool)
    .await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn update_global_config(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<UpdateSafetyConfig>,
) -> Result<ResponseJson<ApiResponse<SafetyConfig>>, ApiError> {
    let pool = &deployment.db().pool;
    let now_str = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    sqlx::query!(
        r#"UPDATE safety_config SET
           require_human_approval = COALESCE(?1, require_human_approval),
           max_concurrent_dispatch = COALESCE(?2, max_concurrent_dispatch),
           max_daily_dispatch = COALESCE(?3, max_daily_dispatch),
           cooldown_seconds = COALESCE(?4, cooldown_seconds),
           auto_approval_enabled = COALESCE(?5, auto_approval_enabled),
           auto_approval_policy = COALESCE(?6, auto_approval_policy),
           auto_approval_model_preset_id = COALESCE(?7, auto_approval_model_preset_id),
           updated_at = ?8
           WHERE scope = 'global'"#,
        payload.require_human_approval,
        payload.max_concurrent_dispatch,
        payload.max_daily_dispatch,
        payload.cooldown_seconds,
        payload.auto_approval_enabled,
        payload.auto_approval_policy,
        payload.auto_approval_model_preset_id,
        now_str
    )
    .execute(pool)
    .await?;
    get_global_config(State(deployment)).await
}

pub async fn check_dispatch_safety(
    State(deployment): State<DeploymentImpl>,
    Path(workspace_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<SafetyCheckResult>>, ApiError> {
    let pool = &deployment.db().pool;

    let config = sqlx::query_as!(
        SafetyConfig,
        r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id?: Uuid",
           scope, require_human_approval AS "require_human_approval!: bool",
           max_concurrent_dispatch AS "max_concurrent_dispatch!: i64",
           max_daily_dispatch AS "max_daily_dispatch!: i64",
           cooldown_seconds AS "cooldown_seconds!: i64",
           auto_approval_enabled AS "auto_approval_enabled!: bool",
           auto_approval_policy,
           auto_approval_model_preset_id AS "auto_approval_model_preset_id?: Uuid",
           created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
           updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
           FROM safety_config
           WHERE (scope = 'workspace' AND workspace_id = ?1)
              OR scope = 'global'
           ORDER BY CASE WHEN scope = 'workspace' THEN 0 ELSE 1 END
           LIMIT 1"#,
        workspace_id
    )
    .fetch_one(pool)
    .await?;

    let active: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM dispatch_log WHERE workspace_id = ?1 AND status IN ('pending','running')",
        workspace_id
    )
    .fetch_one(pool)
    .await?;

    // Both sides must use the same timestamp format. started_at is stored
    // as `YYYY-MM-DDTHH:MM:SS.sssZ` (strftime), so the cutoff has to use
    // the same strftime mask — `datetime(...)` returns a space-separated
    // form which compared lexicographically would silently miscount.
    let daily: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM dispatch_log
         WHERE workspace_id = ?1
           AND started_at > strftime('%Y-%m-%dT%H:%M:%fZ','now','-1 day')",
        workspace_id
    )
    .fetch_one(pool)
    .await?;

    let mut allowed = true;
    let mut reason = None;

    if active >= config.max_concurrent_dispatch {
        allowed = false;
        reason = Some(format!(
            "concurrent limit reached ({}/{})",
            active, config.max_concurrent_dispatch
        ));
    } else if daily >= config.max_daily_dispatch {
        allowed = false;
        reason = Some(format!(
            "daily limit reached ({}/{})",
            daily, config.max_daily_dispatch
        ));
    }

    Ok(ResponseJson(ApiResponse::success(SafetyCheckResult {
        allowed,
        reason,
        active_dispatches: active,
        daily_dispatches: daily,
        require_human_approval: config.require_human_approval,
    })))
}

pub async fn auto_approve(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<AutoApprovalRequest>,
) -> Result<ResponseJson<ApiResponse<AutoApprovalDecision>>, ApiError> {
    let pool = &deployment.db().pool;
    let decision = services::services::auto_approval::evaluate_and_log(
        pool,
        payload.workspace_id,
        &payload.action_kind,
        &payload.action_summary,
        None,
    )
    .await?;
    Ok(ResponseJson(ApiResponse::success(decision)))
}

#[derive(Deserialize)]
pub struct AutoApprovalLogFilter {
    pub workspace_id: Option<Uuid>,
    pub pending_only: Option<bool>,
}

pub async fn list_auto_approval_log(
    State(deployment): State<DeploymentImpl>,
    Query(filter): Query<AutoApprovalLogFilter>,
) -> Result<ResponseJson<ApiResponse<Vec<AutoApprovalLogEntry>>>, ApiError> {
    let pool = &deployment.db().pool;
    let pending_only = filter.pending_only.unwrap_or(false);
    let rows = match (filter.workspace_id, pending_only) {
        (Some(ws), true) => {
            sqlx::query_as!(
                AutoApprovalLogEntry,
                r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id!: Uuid",
                   action_kind, action_summary, decision, reasoning, decided_by,
                   decided_at AS "decided_at!: chrono::DateTime<chrono::Utc>",
                   resolved_decision,
                   resolved_at AS "resolved_at?: chrono::DateTime<chrono::Utc>",
                   approval_id
                   FROM auto_approval_log
                   WHERE workspace_id = ?1 AND decision = 'escalated' AND resolved_decision IS NULL
                   ORDER BY decided_at DESC LIMIT 200"#,
                ws
            )
            .fetch_all(pool)
            .await?
        }
        (Some(ws), false) => {
            sqlx::query_as!(
                AutoApprovalLogEntry,
                r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id!: Uuid",
                   action_kind, action_summary, decision, reasoning, decided_by,
                   decided_at AS "decided_at!: chrono::DateTime<chrono::Utc>",
                   resolved_decision,
                   resolved_at AS "resolved_at?: chrono::DateTime<chrono::Utc>",
                   approval_id
                   FROM auto_approval_log WHERE workspace_id = ?1
                   ORDER BY decided_at DESC LIMIT 200"#,
                ws
            )
            .fetch_all(pool)
            .await?
        }
        (None, true) => {
            sqlx::query_as!(
                AutoApprovalLogEntry,
                r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id!: Uuid",
                   action_kind, action_summary, decision, reasoning, decided_by,
                   decided_at AS "decided_at!: chrono::DateTime<chrono::Utc>",
                   resolved_decision,
                   resolved_at AS "resolved_at?: chrono::DateTime<chrono::Utc>",
                   approval_id
                   FROM auto_approval_log
                   WHERE decision = 'escalated' AND resolved_decision IS NULL
                   ORDER BY decided_at DESC LIMIT 200"#
            )
            .fetch_all(pool)
            .await?
        }
        (None, false) => {
            sqlx::query_as!(
                AutoApprovalLogEntry,
                r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id!: Uuid",
                   action_kind, action_summary, decision, reasoning, decided_by,
                   decided_at AS "decided_at!: chrono::DateTime<chrono::Utc>",
                   resolved_decision,
                   resolved_at AS "resolved_at?: chrono::DateTime<chrono::Utc>",
                   approval_id
                   FROM auto_approval_log
                   ORDER BY decided_at DESC LIMIT 200"#
            )
            .fetch_all(pool)
            .await?
        }
    };
    Ok(ResponseJson(ApiResponse::success(rows)))
}

pub async fn resolve_auto_approval(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    ResponseJson(payload): ResponseJson<ResolveAutoApprovalRequest>,
) -> Result<ResponseJson<ApiResponse<AutoApprovalLogEntry>>, ApiError> {
    if payload.decision != "approved" && payload.decision != "denied" {
        return Err(ApiError::BadRequest(
            "decision must be 'approved' or 'denied'".into(),
        ));
    }
    let pool = &deployment.db().pool;
    let now_str = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    // Idempotent: only the first resolve wins. Two concurrent clicks
    // race here; the loser's UPDATE matches zero rows and we skip the
    // Approvals::respond call so we don't double-resolve the waiter.
    let result = sqlx::query!(
        r#"UPDATE auto_approval_log
           SET resolved_decision = ?1, resolved_at = ?2
           WHERE id = ?3 AND resolved_decision IS NULL"#,
        payload.decision,
        now_str,
        id,
    )
    .execute(pool)
    .await?;
    let we_resolved = result.rows_affected() > 0;

    let row = sqlx::query_as!(
        AutoApprovalLogEntry,
        r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id!: Uuid",
           action_kind, action_summary, decision, reasoning, decided_by,
           decided_at AS "decided_at!: chrono::DateTime<chrono::Utc>",
           resolved_decision,
           resolved_at AS "resolved_at?: chrono::DateTime<chrono::Utc>",
           approval_id
           FROM auto_approval_log WHERE id = ?1"#,
        id
    )
    .fetch_one(pool)
    .await?;

    // Distinguish the two zero-rows-affected cases: already-resolved
    // (the row exists with a non-null resolved_decision) is a 409, not
    // a silent 200. The fetch above tells us which case we're in.
    if !we_resolved && row.resolved_decision.is_some() {
        return Err(ApiError::Conflict(format!(
            "auto-approval entry {id} already resolved as '{}'",
            row.resolved_decision.as_deref().unwrap_or("unknown")
        )));
    }

    if we_resolved && let Some(approval_id) = row.approval_id.as_deref() {
        let outcome = if payload.decision == "approved" {
            utils::approvals::ApprovalOutcome::Approved
        } else {
            utils::approvals::ApprovalOutcome::Denied {
                reason: Some("denied via auto-approval console".to_string()),
            }
        };
        // Approvals::respond looks up execution_process_id from the
        // pending entry — the value supplied here is only echoed back
        // to analytics, so a placeholder is fine.
        let _ = deployment
            .approvals()
            .respond(
                approval_id,
                utils::approvals::ApprovalResponse {
                    execution_process_id: Uuid::nil(),
                    status: outcome,
                },
            )
            .await;
    }

    Ok(ResponseJson(ApiResponse::success(row)))
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/safety/config",
            get(get_global_config).patch(update_global_config),
        )
        .route("/safety/check/{workspace_id}", get(check_dispatch_safety))
        .route("/safety/auto-approve", post(auto_approve))
        .route("/safety/auto-approval-log", get(list_auto_approval_log))
        .route(
            "/safety/auto-approval-log/{id}/resolve",
            post(resolve_auto_approval),
        )
}
