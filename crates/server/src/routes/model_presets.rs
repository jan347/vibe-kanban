use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::model_preset::{CreateModelPreset, ModelPreset, UpdateModelPreset};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub async fn list_presets(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ModelPreset>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rows = sqlx::query_as!(
        ModelPreset,
        r#"
        SELECT
            id                AS "id!: Uuid",
            name,
            description,
            role              AS "role!: db::models::model_preset::ModelPresetRole",
            executor          AS "executor!: db::models::model_preset::ModelPresetExecutor",
            model_id,
            permission_mode,
            reasoning_effort,
            env_vars_json,
            labels_json,
            created_at        AS "created_at!: chrono::DateTime<chrono::Utc>",
            updated_at        AS "updated_at!: chrono::DateTime<chrono::Utc>"
        FROM model_presets
        ORDER BY updated_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;
    Ok(ResponseJson(ApiResponse::success(rows)))
}

pub async fn create_preset(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<CreateModelPreset>,
) -> Result<ResponseJson<ApiResponse<ModelPreset>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO model_presets (
            id, name, description, role, executor, model_id,
            permission_mode, reasoning_effort, env_vars_json, labels_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        id,
        payload.name,
        payload.description,
        payload.role,
        payload.executor,
        payload.model_id,
        payload.permission_mode,
        payload.reasoning_effort,
        payload.env_vars_json,
        payload.labels_json
    )
    .execute(pool)
    .await?;
    let row = fetch_preset(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn get_preset(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<ModelPreset>>, ApiError> {
    let pool = &deployment.db().pool;
    let row = fetch_preset(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn update_preset(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    ResponseJson(payload): ResponseJson<UpdateModelPreset>,
) -> Result<ResponseJson<ApiResponse<ModelPreset>>, ApiError> {
    let pool = &deployment.db().pool;
    let now_str = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    sqlx::query!(
        r#"
        UPDATE model_presets SET
            name             = COALESCE(?1, name),
            description      = COALESCE(?2, description),
            role             = COALESCE(?3, role),
            executor         = COALESCE(?4, executor),
            model_id         = COALESCE(?5, model_id),
            permission_mode  = COALESCE(?6, permission_mode),
            reasoning_effort = COALESCE(?7, reasoning_effort),
            env_vars_json    = COALESCE(?8, env_vars_json),
            labels_json      = COALESCE(?9, labels_json),
            updated_at       = ?10
        WHERE id = ?11
        "#,
        payload.name,
        payload.description,
        payload.role,
        payload.executor,
        payload.model_id,
        payload.permission_mode,
        payload.reasoning_effort,
        payload.env_vars_json,
        payload.labels_json,
        now_str,
        id
    )
    .execute(pool)
    .await?;
    let row = fetch_preset(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn delete_preset(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    sqlx::query!("DELETE FROM model_presets WHERE id = ?1", id)
        .execute(pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fetch_preset(pool: &sqlx::SqlitePool, id: Uuid) -> Result<ModelPreset, ApiError> {
    sqlx::query_as!(
        ModelPreset,
        r#"
        SELECT
            id                AS "id!: Uuid",
            name,
            description,
            role              AS "role!: db::models::model_preset::ModelPresetRole",
            executor          AS "executor!: db::models::model_preset::ModelPresetExecutor",
            model_id,
            permission_mode,
            reasoning_effort,
            env_vars_json,
            labels_json,
            created_at        AS "created_at!: chrono::DateTime<chrono::Utc>",
            updated_at        AS "updated_at!: chrono::DateTime<chrono::Utc>"
        FROM model_presets
        WHERE id = ?1
        "#,
        id
    )
    .fetch_one(pool)
    .await
    .map_err(ApiError::from)
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/model-presets", get(list_presets).post(create_preset))
        .route(
            "/model-presets/{id}",
            get(get_preset).patch(update_preset).delete(delete_preset),
        )
}
