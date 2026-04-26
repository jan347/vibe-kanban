use db::{
    DBService,
    models::work_item::{WorkItem, WorkItemRun, WorkItemStatus},
};
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool,
    tool_router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::{McpServer, ToolError};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateWorkItemRequest {
    #[schemars(description = "Work item title")]
    title: String,
    #[schemars(description = "Work item description (markdown)")]
    description: Option<String>,
    #[schemars(description = "Priority (0 = lowest)")]
    priority: Option<i64>,
    #[schemars(description = "Comma-separated tags JSON array")]
    tags_json: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ListWorkItemsRequest {
    #[schemars(description = "Filter by status: open, in_progress, review, done, cancelled")]
    status: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetWorkItemRequest {
    #[schemars(description = "Work item UUID")]
    id: Uuid,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct UpdateWorkItemRequest {
    #[schemars(description = "Work item UUID")]
    id: Uuid,
    #[schemars(description = "New title")]
    title: Option<String>,
    #[schemars(description = "New description")]
    description: Option<String>,
    #[schemars(description = "New status: open, in_progress, review, done, cancelled")]
    status: Option<String>,
    #[schemars(description = "New priority")]
    priority: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LinkWorkspaceRequest {
    #[schemars(description = "Work item UUID")]
    work_item_id: Uuid,
    #[schemars(description = "Workspace UUID to link")]
    workspace_id: Option<Uuid>,
    #[schemars(description = "Role label (e.g. 'implementer', 'reviewer')")]
    role: Option<String>,
}

#[tool_router(router = work_item_tools_router, vis = "pub")]
impl McpServer {
    #[tool(
        name = "work_item.create",
        description = "Create a new work item for orchestrating agent tasks."
    )]
    async fn work_item_create(
        &self,
        Parameters(req): Parameters<CreateWorkItemRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match work_item_db().await {
            Ok(db) => db,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        let id = Uuid::new_v4();
        let status = WorkItemStatus::Open;
        let priority = req.priority.unwrap_or(0);
        let result = sqlx::query!(
            r#"INSERT INTO work_items (id, title, description, status, priority, tags_json)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            id,
            req.title,
            req.description,
            status,
            priority,
            req.tags_json
        )
        .execute(&db.pool)
        .await;
        match result {
            Ok(_) => match fetch_work_item(&db, id).await {
                Ok(wi) => Self::success(&wi),
                Err(e) => Ok(Self::tool_error(e)),
            },
            Err(e) => Ok(Self::tool_error(ToolError::new(
                "DB error",
                Some(e.to_string()),
            ))),
        }
    }

    #[tool(
        name = "work_item.list",
        description = "List work items, optionally filtered by status."
    )]
    async fn work_item_list(
        &self,
        Parameters(req): Parameters<ListWorkItemsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match work_item_db().await {
            Ok(db) => db,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        let rows = if let Some(status_str) = &req.status {
            sqlx::query_as!(
                WorkItem,
                r#"SELECT id AS "id!: Uuid", title, description,
                   status AS "status!: WorkItemStatus", priority AS "priority!: i64",
                   tags_json, created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
                   updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
                   FROM work_items WHERE status = ?1
                   ORDER BY priority DESC, updated_at DESC"#,
                status_str
            )
            .fetch_all(&db.pool)
            .await
        } else {
            sqlx::query_as!(
                WorkItem,
                r#"SELECT id AS "id!: Uuid", title, description,
                   status AS "status!: WorkItemStatus", priority AS "priority!: i64",
                   tags_json, created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
                   updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
                   FROM work_items ORDER BY priority DESC, updated_at DESC"#
            )
            .fetch_all(&db.pool)
            .await
        };
        match rows {
            Ok(items) => Self::success(&items),
            Err(e) => Ok(Self::tool_error(ToolError::new(
                "DB error",
                Some(e.to_string()),
            ))),
        }
    }

    #[tool(
        name = "work_item.get",
        description = "Get a work item by ID with linked workspace runs."
    )]
    async fn work_item_get(
        &self,
        Parameters(req): Parameters<GetWorkItemRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match work_item_db().await {
            Ok(db) => db,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        let wi = match fetch_work_item(&db, req.id).await {
            Ok(wi) => wi,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        let runs = sqlx::query_as!(
            WorkItemRun,
            r#"SELECT work_item_id AS "work_item_id!: Uuid",
               workspace_id AS "workspace_id!: Uuid", role,
               created_at AS "created_at!: chrono::DateTime<chrono::Utc>"
               FROM work_item_runs WHERE work_item_id = ?1
               ORDER BY created_at ASC"#,
            req.id
        )
        .fetch_all(&db.pool)
        .await
        .unwrap_or_default();

        #[derive(serde::Serialize)]
        struct WorkItemDetail {
            work_item: WorkItem,
            linked_runs: Vec<WorkItemRun>,
        }
        Self::success(&WorkItemDetail {
            work_item: wi,
            linked_runs: runs,
        })
    }

    #[tool(
        name = "work_item.update",
        description = "Update a work item's title, description, status, or priority."
    )]
    async fn work_item_update(
        &self,
        Parameters(req): Parameters<UpdateWorkItemRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match work_item_db().await {
            Ok(db) => db,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        let now_str = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let result = sqlx::query!(
            r#"UPDATE work_items SET
               title = COALESCE(?1, title),
               description = COALESCE(?2, description),
               status = COALESCE(?3, status),
               priority = COALESCE(?4, priority),
               updated_at = ?5
               WHERE id = ?6"#,
            req.title,
            req.description,
            req.status,
            req.priority,
            now_str,
            req.id
        )
        .execute(&db.pool)
        .await;
        match result {
            Ok(_) => match fetch_work_item(&db, req.id).await {
                Ok(wi) => Self::success(&wi),
                Err(e) => Ok(Self::tool_error(e)),
            },
            Err(e) => Ok(Self::tool_error(ToolError::new(
                "DB error",
                Some(e.to_string()),
            ))),
        }
    }

    #[tool(
        name = "work_item.link_workspace",
        description = "Link a workspace to a work item so agents working in that workspace contribute to the work item."
    )]
    async fn work_item_link_workspace(
        &self,
        Parameters(req): Parameters<LinkWorkspaceRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let workspace_id = match self.resolve_workspace_id(req.workspace_id) {
            Ok(id) => id,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        let db = match work_item_db().await {
            Ok(db) => db,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        let result = sqlx::query!(
            r#"INSERT OR IGNORE INTO work_item_runs (work_item_id, workspace_id, role)
               VALUES (?1, ?2, ?3)"#,
            req.work_item_id,
            workspace_id,
            req.role
        )
        .execute(&db.pool)
        .await;
        match result {
            Ok(_) => Self::success(
                &serde_json::json!({"ok": true, "work_item_id": req.work_item_id.to_string(), "workspace_id": workspace_id.to_string()}),
            ),
            Err(e) => Ok(Self::tool_error(ToolError::new(
                "DB error",
                Some(e.to_string()),
            ))),
        }
    }
}

async fn work_item_db() -> Result<DBService, ToolError> {
    DBService::new()
        .await
        .map_err(|e| ToolError::new("Failed to open VK database", Some(e.to_string())))
}

async fn fetch_work_item(db: &DBService, id: Uuid) -> Result<WorkItem, ToolError> {
    sqlx::query_as!(
        WorkItem,
        r#"SELECT id AS "id!: Uuid", title, description,
           status AS "status!: WorkItemStatus", priority AS "priority!: i64",
           tags_json, created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
           updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
           FROM work_items WHERE id = ?1"#,
        id
    )
    .fetch_one(&db.pool)
    .await
    .map_err(|e| ToolError::new("work item not found", Some(e.to_string())))
}
