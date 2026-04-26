use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::work_item::{
    CreateWorkItem, LinkWorkspaceToWorkItem, UpdateWorkItem, WorkItem, WorkItemRun,
    WorkItemStatus, WorkItemWithLinks,
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub async fn list_work_items(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<WorkItem>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rows = sqlx::query_as!(
        WorkItem,
        r#"
        SELECT
            id          AS "id!: Uuid",
            title,
            description,
            status      AS "status!: WorkItemStatus",
            priority    AS "priority!: i64",
            tags_json,
            created_at  AS "created_at!: chrono::DateTime<chrono::Utc>",
            updated_at  AS "updated_at!: chrono::DateTime<chrono::Utc>"
        FROM work_items
        ORDER BY priority DESC, updated_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;
    Ok(ResponseJson(ApiResponse::success(rows)))
}

pub async fn create_work_item(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<CreateWorkItem>,
) -> Result<ResponseJson<ApiResponse<WorkItem>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();
    let status = payload.status.unwrap_or(WorkItemStatus::Open);
    let priority = payload.priority.unwrap_or(0);
    sqlx::query!(
        r#"
        INSERT INTO work_items (id, title, description, status, priority, tags_json)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        id,
        payload.title,
        payload.description,
        status,
        priority,
        payload.tags_json
    )
    .execute(pool)
    .await?;
    let row = fetch_work_item(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn get_work_item(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<WorkItemWithLinks>>, ApiError> {
    let pool = &deployment.db().pool;
    let work_item = fetch_work_item(pool, id).await?;
    let linked_runs = sqlx::query_as!(
        WorkItemRun,
        r#"
        SELECT
            work_item_id AS "work_item_id!: Uuid",
            workspace_id AS "workspace_id!: Uuid",
            role,
            created_at   AS "created_at!: chrono::DateTime<chrono::Utc>"
        FROM work_item_runs
        WHERE work_item_id = ?1
        ORDER BY created_at ASC
        "#,
        id
    )
    .fetch_all(pool)
    .await?;
    Ok(ResponseJson(ApiResponse::success(WorkItemWithLinks {
        work_item,
        linked_runs,
    })))
}

pub async fn update_work_item(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    ResponseJson(payload): ResponseJson<UpdateWorkItem>,
) -> Result<ResponseJson<ApiResponse<WorkItem>>, ApiError> {
    let pool = &deployment.db().pool;
    let now_str = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    sqlx::query!(
        r#"
        UPDATE work_items SET
            title       = COALESCE(?1, title),
            description = COALESCE(?2, description),
            status      = COALESCE(?3, status),
            priority    = COALESCE(?4, priority),
            tags_json   = COALESCE(?5, tags_json),
            updated_at  = ?6
        WHERE id = ?7
        "#,
        payload.title,
        payload.description,
        payload.status,
        payload.priority,
        payload.tags_json,
        now_str,
        id
    )
    .execute(pool)
    .await?;
    let row = fetch_work_item(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn delete_work_item(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    sqlx::query!("DELETE FROM work_items WHERE id = ?1", id)
        .execute(pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn link_workspace(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    ResponseJson(payload): ResponseJson<LinkWorkspaceToWorkItem>,
) -> Result<ResponseJson<ApiResponse<WorkItemRun>>, ApiError> {
    let pool = &deployment.db().pool;
    sqlx::query!(
        r#"
        INSERT OR IGNORE INTO work_item_runs (work_item_id, workspace_id, role)
        VALUES (?1, ?2, ?3)
        "#,
        id,
        payload.workspace_id,
        payload.role
    )
    .execute(pool)
    .await?;
    let row = sqlx::query_as!(
        WorkItemRun,
        r#"
        SELECT
            work_item_id AS "work_item_id!: Uuid",
            workspace_id AS "workspace_id!: Uuid",
            role,
            created_at   AS "created_at!: chrono::DateTime<chrono::Utc>"
        FROM work_item_runs
        WHERE work_item_id = ?1 AND workspace_id = ?2
        "#,
        id,
        payload.workspace_id
    )
    .fetch_one(pool)
    .await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn unlink_workspace(
    State(deployment): State<DeploymentImpl>,
    Path((id, workspace_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    sqlx::query!(
        "DELETE FROM work_item_runs WHERE work_item_id = ?1 AND workspace_id = ?2",
        id,
        workspace_id
    )
    .execute(pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fetch_work_item(
    pool: &sqlx::SqlitePool,
    id: Uuid,
) -> Result<WorkItem, ApiError> {
    sqlx::query_as!(
        WorkItem,
        r#"
        SELECT
            id          AS "id!: Uuid",
            title,
            description,
            status      AS "status!: WorkItemStatus",
            priority    AS "priority!: i64",
            tags_json,
            created_at  AS "created_at!: chrono::DateTime<chrono::Utc>",
            updated_at  AS "updated_at!: chrono::DateTime<chrono::Utc>"
        FROM work_items
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
        .route(
            "/work-items",
            get(list_work_items).post(create_work_item),
        )
        .route(
            "/work-items/{id}",
            get(get_work_item)
                .patch(update_work_item)
                .delete(delete_work_item),
        )
        .route("/work-items/{id}/links", get(get_work_item).post(link_workspace))
        .route(
            "/work-items/{id}/links/{workspace_id}",
            get(get_work_item).delete(unlink_workspace),
        )
}
