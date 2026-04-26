use axum::{
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::dispatch::{CreateDispatch, DispatchLogEntry, DispatchStatus};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

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
) -> Result<ResponseJson<ApiResponse<DispatchLogEntry>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();
    let status = DispatchStatus::Pending;
    sqlx::query!(
        r#"INSERT INTO dispatch_log (id, work_item_id, workspace_id, prompt_text,
           prompt_template_id, model_preset_id, status)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
        id,
        payload.work_item_id,
        payload.workspace_id,
        payload.prompt_text,
        payload.prompt_template_id,
        payload.model_preset_id,
        status
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        "INSERT OR IGNORE INTO work_item_runs (work_item_id, workspace_id, role) VALUES (?1, ?2, 'dispatch')",
        payload.work_item_id, payload.workspace_id
    )
    .execute(pool)
    .await?;
    let row = fetch_dispatch(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
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
