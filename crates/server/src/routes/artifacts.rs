use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::artifact::{
    AcceptArtifactRequest, Artifact, ArtifactKind, CreateArtifact, SharedContextItem,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Deserialize)]
pub struct ArtifactFilter {
    pub work_item_id: Option<Uuid>,
    pub kind: Option<ArtifactKind>,
}

pub async fn list_artifacts(
    State(deployment): State<DeploymentImpl>,
    Query(filter): Query<ArtifactFilter>,
) -> Result<ResponseJson<ApiResponse<Vec<Artifact>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rows = match (&filter.work_item_id, &filter.kind) {
        (Some(wi), Some(k)) => {
            sqlx::query_as!(
                Artifact,
                r#"
                SELECT
                    id                              AS "id!: Uuid",
                    work_item_id                    AS "work_item_id?: Uuid",
                    kind                            AS "kind!: ArtifactKind",
                    title,
                    body_text,
                    body_json,
                    version                         AS "version!: i64",
                    parent_artifact_id              AS "parent_artifact_id?: Uuid",
                    created_by_kind,
                    created_by_workspace_id         AS "created_by_workspace_id?: Uuid",
                    created_by_execution_process_id AS "created_by_execution_process_id?: Uuid",
                    created_at                      AS "created_at!: chrono::DateTime<chrono::Utc>",
                    accepted_at                     AS "accepted_at?: chrono::DateTime<chrono::Utc>"
                FROM artifacts
                WHERE work_item_id = ?1 AND kind = ?2
                ORDER BY created_at DESC
                "#,
                wi,
                k
            )
            .fetch_all(pool)
            .await?
        }
        (Some(wi), None) => {
            sqlx::query_as!(
                Artifact,
                r#"
                SELECT
                    id                              AS "id!: Uuid",
                    work_item_id                    AS "work_item_id?: Uuid",
                    kind                            AS "kind!: ArtifactKind",
                    title,
                    body_text,
                    body_json,
                    version                         AS "version!: i64",
                    parent_artifact_id              AS "parent_artifact_id?: Uuid",
                    created_by_kind,
                    created_by_workspace_id         AS "created_by_workspace_id?: Uuid",
                    created_by_execution_process_id AS "created_by_execution_process_id?: Uuid",
                    created_at                      AS "created_at!: chrono::DateTime<chrono::Utc>",
                    accepted_at                     AS "accepted_at?: chrono::DateTime<chrono::Utc>"
                FROM artifacts
                WHERE work_item_id = ?1
                ORDER BY created_at DESC
                "#,
                wi
            )
            .fetch_all(pool)
            .await?
        }
        (None, Some(k)) => {
            sqlx::query_as!(
                Artifact,
                r#"
                SELECT
                    id                              AS "id!: Uuid",
                    work_item_id                    AS "work_item_id?: Uuid",
                    kind                            AS "kind!: ArtifactKind",
                    title,
                    body_text,
                    body_json,
                    version                         AS "version!: i64",
                    parent_artifact_id              AS "parent_artifact_id?: Uuid",
                    created_by_kind,
                    created_by_workspace_id         AS "created_by_workspace_id?: Uuid",
                    created_by_execution_process_id AS "created_by_execution_process_id?: Uuid",
                    created_at                      AS "created_at!: chrono::DateTime<chrono::Utc>",
                    accepted_at                     AS "accepted_at?: chrono::DateTime<chrono::Utc>"
                FROM artifacts
                WHERE kind = ?1
                ORDER BY created_at DESC
                "#,
                k
            )
            .fetch_all(pool)
            .await?
        }
        (None, None) => {
            sqlx::query_as!(
                Artifact,
                r#"
                SELECT
                    id                              AS "id!: Uuid",
                    work_item_id                    AS "work_item_id?: Uuid",
                    kind                            AS "kind!: ArtifactKind",
                    title,
                    body_text,
                    body_json,
                    version                         AS "version!: i64",
                    parent_artifact_id              AS "parent_artifact_id?: Uuid",
                    created_by_kind,
                    created_by_workspace_id         AS "created_by_workspace_id?: Uuid",
                    created_by_execution_process_id AS "created_by_execution_process_id?: Uuid",
                    created_at                      AS "created_at!: chrono::DateTime<chrono::Utc>",
                    accepted_at                     AS "accepted_at?: chrono::DateTime<chrono::Utc>"
                FROM artifacts
                ORDER BY created_at DESC
                "#
            )
            .fetch_all(pool)
            .await?
        }
    };
    Ok(ResponseJson(ApiResponse::success(rows)))
}

pub async fn create_artifact(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<CreateArtifact>,
) -> Result<ResponseJson<ApiResponse<Artifact>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO artifacts (id, work_item_id, kind, title, body_text, body_json,
            parent_artifact_id, created_by_kind, created_by_workspace_id,
            created_by_execution_process_id)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        id,
        payload.work_item_id,
        payload.kind,
        payload.title,
        payload.body_text,
        payload.body_json,
        payload.parent_artifact_id,
        payload.created_by_kind,
        payload.created_by_workspace_id,
        payload.created_by_execution_process_id
    )
    .execute(pool)
    .await?;
    let row = fetch_artifact(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn get_artifact(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Artifact>>, ApiError> {
    let pool = &deployment.db().pool;
    let row = fetch_artifact(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn accept_artifact(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    ResponseJson(payload): ResponseJson<AcceptArtifactRequest>,
) -> Result<ResponseJson<ApiResponse<Artifact>>, ApiError> {
    let pool = &deployment.db().pool;
    if payload.accepted {
        let now_str = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        sqlx::query!(
            "UPDATE artifacts SET accepted_at = ?1 WHERE id = ?2",
            now_str,
            id
        )
        .execute(pool)
        .await?;
    } else {
        sqlx::query!("UPDATE artifacts SET accepted_at = NULL WHERE id = ?1", id)
            .execute(pool)
            .await?;
    }
    let row = fetch_artifact(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn delete_artifact(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    sqlx::query!("DELETE FROM artifacts WHERE id = ?1", id)
        .execute(pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn shared_context(
    State(deployment): State<DeploymentImpl>,
    Path(work_item_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<SharedContextItem>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rows = sqlx::query_as!(
        SharedContextItem,
        r#"
        SELECT
            work_item_id    AS "work_item_id!: Uuid",
            context_type    AS "context_type!: String",
            context_id      AS "context_id!: Uuid",
            context_title   AS "context_title: String",
            context_body    AS "context_body: String",
            context_at      AS "context_at?: chrono::DateTime<chrono::Utc>"
        FROM work_item_shared_context
        WHERE work_item_id = ?1
        ORDER BY context_at DESC
        "#,
        work_item_id
    )
    .fetch_all(pool)
    .await?;
    Ok(ResponseJson(ApiResponse::success(rows)))
}

async fn fetch_artifact(pool: &sqlx::SqlitePool, id: Uuid) -> Result<Artifact, ApiError> {
    sqlx::query_as!(
        Artifact,
        r#"
        SELECT
            id                              AS "id!: Uuid",
            work_item_id                    AS "work_item_id?: Uuid",
            kind                            AS "kind!: ArtifactKind",
            title,
            body_text,
            body_json,
            version                         AS "version!: i64",
            parent_artifact_id              AS "parent_artifact_id?: Uuid",
            created_by_kind,
            created_by_workspace_id         AS "created_by_workspace_id?: Uuid",
            created_by_execution_process_id AS "created_by_execution_process_id?: Uuid",
            created_at                      AS "created_at!: chrono::DateTime<chrono::Utc>",
            accepted_at                     AS "accepted_at?: chrono::DateTime<chrono::Utc>"
        FROM artifacts
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
        .route("/artifacts", get(list_artifacts).post(create_artifact))
        .route("/artifacts/{id}", get(get_artifact).delete(delete_artifact))
        .route(
            "/artifacts/{id}/accept",
            axum::routing::post(accept_artifact),
        )
        .route(
            "/work-items/{work_item_id}/shared-context",
            get(shared_context),
        )
}
