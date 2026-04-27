use axum::{
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::{
    dispatch::{CreateDispatch, DispatchLogEntry, DispatchStatus},
    safety::AutoApprovalDecision,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::dispatch_guard::{
    GatedDispatchError, GatedDispatchResult, gated_create_dispatch,
};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// Create-dispatch returns either the freshly inserted dispatch row OR
/// the supervisor decision that blocked it. The frontend distinguishes
/// the two via the `status` field.
#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CreateDispatchResponse {
    Approved { dispatch: DispatchLogEntry },
    Blocked { decision: AutoApprovalDecision },
}

impl From<GatedDispatchError> for ApiError {
    fn from(e: GatedDispatchError) -> Self {
        match e {
            GatedDispatchError::WorkspaceNotFound(id) => {
                ApiError::BadRequest(format!("workspace not found: {id}"))
            }
            GatedDispatchError::Db(err) => ApiError::Database(err),
        }
    }
}

#[derive(Deserialize)]
pub struct DispatchFilter {
    pub work_item_id: Option<Uuid>,
    pub status: Option<String>,
}

pub async fn list_dispatches(
    State(deployment): State<DeploymentImpl>,
    Query(filter): Query<DispatchFilter>,
) -> Result<ResponseJson<ApiResponse<Vec<DispatchLogEntry>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rows = match (&filter.work_item_id, &filter.status) {
        (Some(wi), Some(s)) => {
            sqlx::query_as!(
                DispatchLogEntry,
                r#"SELECT id AS "id!: Uuid", work_item_id AS "work_item_id!: Uuid",
                   workspace_id AS "workspace_id!: Uuid", session_id AS "session_id?: Uuid",
                   prompt_template_id AS "prompt_template_id?: Uuid", prompt_text,
                   model_preset_id AS "model_preset_id?: Uuid",
                   status AS "status!: DispatchStatus",
                   started_at AS "started_at!: chrono::DateTime<chrono::Utc>",
                   completed_at AS "completed_at?: chrono::DateTime<chrono::Utc>",
                   error_message
                   FROM dispatch_log WHERE work_item_id = ?1 AND status = ?2
                   ORDER BY started_at DESC"#,
                wi,
                s
            )
            .fetch_all(pool)
            .await?
        }
        (Some(wi), None) => {
            sqlx::query_as!(
                DispatchLogEntry,
                r#"SELECT id AS "id!: Uuid", work_item_id AS "work_item_id!: Uuid",
                   workspace_id AS "workspace_id!: Uuid", session_id AS "session_id?: Uuid",
                   prompt_template_id AS "prompt_template_id?: Uuid", prompt_text,
                   model_preset_id AS "model_preset_id?: Uuid",
                   status AS "status!: DispatchStatus",
                   started_at AS "started_at!: chrono::DateTime<chrono::Utc>",
                   completed_at AS "completed_at?: chrono::DateTime<chrono::Utc>",
                   error_message
                   FROM dispatch_log WHERE work_item_id = ?1
                   ORDER BY started_at DESC"#,
                wi
            )
            .fetch_all(pool)
            .await?
        }
        (None, Some(s)) => {
            sqlx::query_as!(
                DispatchLogEntry,
                r#"SELECT id AS "id!: Uuid", work_item_id AS "work_item_id!: Uuid",
                   workspace_id AS "workspace_id!: Uuid", session_id AS "session_id?: Uuid",
                   prompt_template_id AS "prompt_template_id?: Uuid", prompt_text,
                   model_preset_id AS "model_preset_id?: Uuid",
                   status AS "status!: DispatchStatus",
                   started_at AS "started_at!: chrono::DateTime<chrono::Utc>",
                   completed_at AS "completed_at?: chrono::DateTime<chrono::Utc>",
                   error_message
                   FROM dispatch_log WHERE status = ?1
                   ORDER BY started_at DESC"#,
                s
            )
            .fetch_all(pool)
            .await?
        }
        (None, None) => {
            sqlx::query_as!(
                DispatchLogEntry,
                r#"SELECT id AS "id!: Uuid", work_item_id AS "work_item_id!: Uuid",
                   workspace_id AS "workspace_id!: Uuid", session_id AS "session_id?: Uuid",
                   prompt_template_id AS "prompt_template_id?: Uuid", prompt_text,
                   model_preset_id AS "model_preset_id?: Uuid",
                   status AS "status!: DispatchStatus",
                   started_at AS "started_at!: chrono::DateTime<chrono::Utc>",
                   completed_at AS "completed_at?: chrono::DateTime<chrono::Utc>",
                   error_message
                   FROM dispatch_log ORDER BY started_at DESC"#
            )
            .fetch_all(pool)
            .await?
        }
    };
    Ok(ResponseJson(ApiResponse::success(rows)))
}

pub async fn create_dispatch(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<CreateDispatch>,
) -> Result<ResponseJson<ApiResponse<CreateDispatchResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    // Cap prompt size at the boundary so a runaway agent can't write an
    // arbitrarily large row through the dispatch endpoint.
    if payload.prompt_text.len() > 64 * 1024 {
        return Err(ApiError::BadRequest("prompt_text exceeds 64KB".to_string()));
    }
    // Summary fed to the supervisor — first line of the prompt is the
    // signal-rich part; truncate to keep audit logs tidy.
    let summary: String = payload
        .prompt_text
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(280)
        .collect();
    let response = match gated_create_dispatch(pool, &payload, "dispatch", &summary).await? {
        GatedDispatchResult::Approved(row) => CreateDispatchResponse::Approved { dispatch: row },
        GatedDispatchResult::Blocked(decision) => CreateDispatchResponse::Blocked { decision },
    };
    Ok(ResponseJson(ApiResponse::success(response)))
}

pub async fn get_dispatch(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<DispatchLogEntry>>, ApiError> {
    let pool = &deployment.db().pool;
    let row = fetch_dispatch(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

async fn fetch_dispatch(pool: &sqlx::SqlitePool, id: Uuid) -> Result<DispatchLogEntry, ApiError> {
    sqlx::query_as!(
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
    .fetch_one(pool)
    .await
    .map_err(ApiError::from)
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/dispatches", get(list_dispatches).post(create_dispatch))
        .route("/dispatches/{id}", get(get_dispatch))
}
