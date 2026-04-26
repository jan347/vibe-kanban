use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::safety::{
    AutoApprovalDecision, AutoApprovalRequest, SafetyCheckResult, SafetyConfig, UpdateSafetyConfig,
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

const SAFETY_SELECT: &str = r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id?: Uuid",
    scope, require_human_approval AS "require_human_approval!: bool",
    max_concurrent_dispatch AS "max_concurrent_dispatch!: i64",
    max_daily_dispatch AS "max_daily_dispatch!: i64",
    cooldown_seconds AS "cooldown_seconds!: i64",
    auto_approval_enabled AS "auto_approval_enabled!: bool",
    auto_approval_policy,
    auto_approval_model_preset_id AS "auto_approval_model_preset_id?: Uuid",
    created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
    updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
    FROM safety_config"#;

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
    let _ = SAFETY_SELECT;
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

    let daily: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM dispatch_log WHERE workspace_id = ?1 AND started_at > datetime('now', '-1 day')",
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
           WHERE (scope = 'workspace' AND workspace_id = ?1) OR scope = 'global'
           ORDER BY CASE WHEN scope = 'workspace' THEN 0 ELSE 1 END
           LIMIT 1"#,
        payload.workspace_id,
    )
    .fetch_one(pool)
    .await?;

    let decision = services::services::auto_approval::evaluate(&config, &payload);

    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO auto_approval_log
           (id, workspace_id, action_kind, action_summary, decision, reasoning, decided_by)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
        id,
        payload.workspace_id,
        payload.action_kind,
        payload.action_summary,
        decision.decision,
        decision.reasoning,
        decision.decided_by,
    )
    .execute(pool)
    .await?;

    Ok(ResponseJson(ApiResponse::success(decision)))
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/safety/config",
            get(get_global_config).patch(update_global_config),
        )
        .route("/safety/check/{workspace_id}", get(check_dispatch_safety))
        .route("/safety/auto-approve", post(auto_approve))
}
