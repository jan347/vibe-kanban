use std::collections::HashMap;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::repo_bundle::{
    CreateRepoBundle, RepoBundle, UpdateRepoBundle,
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

fn serialize_repo_ids(ids: &[Uuid]) -> Result<String, ApiError> {
    serde_json::to_string(ids).map_err(|e| ApiError::BadRequest(e.to_string()))
}

fn serialize_branch_overrides(
    map: &Option<HashMap<Uuid, String>>,
) -> Result<Option<String>, ApiError> {
    match map {
        Some(m) => serde_json::to_string(m)
            .map(Some)
            .map_err(|e| ApiError::BadRequest(e.to_string())),
        None => Ok(None),
    }
}

pub async fn list_bundles(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<RepoBundle>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rows = sqlx::query_as!(
        RepoBundle,
        r#"
        SELECT
            id                              AS "id!: Uuid",
            name,
            description,
            repo_ids_json,
            default_branch_overrides_json,
            default_preset_id               AS "default_preset_id?: Uuid",
            created_at                      AS "created_at!: chrono::DateTime<chrono::Utc>",
            updated_at                      AS "updated_at!: chrono::DateTime<chrono::Utc>"
        FROM repo_bundles
        ORDER BY updated_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;
    Ok(ResponseJson(ApiResponse::success(rows)))
}

pub async fn create_bundle(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<CreateRepoBundle>,
) -> Result<ResponseJson<ApiResponse<RepoBundle>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();
    let repo_ids_json = serialize_repo_ids(&payload.repo_ids)?;
    let overrides_json = serialize_branch_overrides(&payload.default_branch_overrides)?;
    sqlx::query!(
        r#"
        INSERT INTO repo_bundles (
            id, name, description, repo_ids_json,
            default_branch_overrides_json, default_preset_id
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        id,
        payload.name,
        payload.description,
        repo_ids_json,
        overrides_json,
        payload.default_preset_id
    )
    .execute(pool)
    .await?;
    let row = fetch_bundle(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn get_bundle(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<RepoBundle>>, ApiError> {
    let pool = &deployment.db().pool;
    let row = fetch_bundle(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn update_bundle(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    ResponseJson(payload): ResponseJson<UpdateRepoBundle>,
) -> Result<ResponseJson<ApiResponse<RepoBundle>>, ApiError> {
    let pool = &deployment.db().pool;
    let now_str = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let repo_ids_json = match &payload.repo_ids {
        Some(ids) => Some(serialize_repo_ids(ids)?),
        None => None,
    };
    let overrides_json = serialize_branch_overrides(&payload.default_branch_overrides)?;
    sqlx::query!(
        r#"
        UPDATE repo_bundles SET
            name                          = COALESCE(?1, name),
            description                   = COALESCE(?2, description),
            repo_ids_json                 = COALESCE(?3, repo_ids_json),
            default_branch_overrides_json = COALESCE(?4, default_branch_overrides_json),
            default_preset_id             = COALESCE(?5, default_preset_id),
            updated_at                    = ?6
        WHERE id = ?7
        "#,
        payload.name,
        payload.description,
        repo_ids_json,
        overrides_json,
        payload.default_preset_id,
        now_str,
        id
    )
    .execute(pool)
    .await?;
    let row = fetch_bundle(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn delete_bundle(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    sqlx::query!("DELETE FROM repo_bundles WHERE id = ?1", id)
        .execute(pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fetch_bundle(
    pool: &sqlx::SqlitePool,
    id: Uuid,
) -> Result<RepoBundle, ApiError> {
    sqlx::query_as!(
        RepoBundle,
        r#"
        SELECT
            id                              AS "id!: Uuid",
            name,
            description,
            repo_ids_json,
            default_branch_overrides_json,
            default_preset_id               AS "default_preset_id?: Uuid",
            created_at                      AS "created_at!: chrono::DateTime<chrono::Utc>",
            updated_at                      AS "updated_at!: chrono::DateTime<chrono::Utc>"
        FROM repo_bundles
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
        .route("/repo-bundles", get(list_bundles).post(create_bundle))
        .route(
            "/repo-bundles/{id}",
            get(get_bundle).patch(update_bundle).delete(delete_bundle),
        )
}
