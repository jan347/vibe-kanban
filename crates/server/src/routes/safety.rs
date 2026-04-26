use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::safety::{SafetyCheckResult, SafetyConfig, UpdateSafetyConfig};
use deployment::Deployment;
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
           updated_at = ?5
           WHERE scope = 'global'"#,
        payload.require_human_approval,
        payload.max_concurrent_dispatch,
        payload.max_daily_dispatch,
        payload.cooldown_seconds,
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

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/safety/config",
            get(get_global_config).patch(update_global_config),
        )
        .route("/safety/check/{workspace_id}", get(check_dispatch_safety))
}
