use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::prompt_template::{
    CreatePromptTemplate, PromptTemplate, PromptTemplateRole, UpdatePromptTemplate,
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub async fn list_templates(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<PromptTemplate>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rows = sqlx::query_as!(
        PromptTemplate,
        r#"
        SELECT
            id          AS "id!: Uuid",
            name,
            role        AS "role!: PromptTemplateRole",
            description,
            body_text,
            preset_id   AS "preset_id?: Uuid",
            bundle_id   AS "bundle_id?: Uuid",
            tags_json,
            created_at  AS "created_at!: chrono::DateTime<chrono::Utc>",
            updated_at  AS "updated_at!: chrono::DateTime<chrono::Utc>"
        FROM prompt_templates
        ORDER BY updated_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;
    Ok(ResponseJson(ApiResponse::success(rows)))
}

pub async fn create_template(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<CreatePromptTemplate>,
) -> Result<ResponseJson<ApiResponse<PromptTemplate>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO prompt_templates (
            id, name, role, description, body_text,
            preset_id, bundle_id, tags_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        id,
        payload.name,
        payload.role,
        payload.description,
        payload.body_text,
        payload.preset_id,
        payload.bundle_id,
        payload.tags_json
    )
    .execute(pool)
    .await?;
    let row = fetch_template(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn get_template(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<PromptTemplate>>, ApiError> {
    let pool = &deployment.db().pool;
    let row = fetch_template(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn update_template(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    ResponseJson(payload): ResponseJson<UpdatePromptTemplate>,
) -> Result<ResponseJson<ApiResponse<PromptTemplate>>, ApiError> {
    let pool = &deployment.db().pool;
    let now_str = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    sqlx::query!(
        r#"
        UPDATE prompt_templates SET
            name        = COALESCE(?1, name),
            role        = COALESCE(?2, role),
            description = COALESCE(?3, description),
            body_text   = COALESCE(?4, body_text),
            preset_id   = COALESCE(?5, preset_id),
            bundle_id   = COALESCE(?6, bundle_id),
            tags_json   = COALESCE(?7, tags_json),
            updated_at  = ?8
        WHERE id = ?9
        "#,
        payload.name,
        payload.role,
        payload.description,
        payload.body_text,
        payload.preset_id,
        payload.bundle_id,
        payload.tags_json,
        now_str,
        id
    )
    .execute(pool)
    .await?;
    let row = fetch_template(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn delete_template(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    sqlx::query!("DELETE FROM prompt_templates WHERE id = ?1", id)
        .execute(pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fetch_template(
    pool: &sqlx::SqlitePool,
    id: Uuid,
) -> Result<PromptTemplate, ApiError> {
    sqlx::query_as!(
        PromptTemplate,
        r#"
        SELECT
            id          AS "id!: Uuid",
            name,
            role        AS "role!: PromptTemplateRole",
            description,
            body_text,
            preset_id   AS "preset_id?: Uuid",
            bundle_id   AS "bundle_id?: Uuid",
            tags_json,
            created_at  AS "created_at!: chrono::DateTime<chrono::Utc>",
            updated_at  AS "updated_at!: chrono::DateTime<chrono::Utc>"
        FROM prompt_templates
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
        .route("/prompt-templates", get(list_templates).post(create_template))
        .route(
            "/prompt-templates/{id}",
            get(get_template).patch(update_template).delete(delete_template),
        )
}
