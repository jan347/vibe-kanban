use db::{
    DBService,
    models::artifact::{Artifact, ArtifactKind},
};
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool,
    tool_router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::{McpServer, ToolError};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateArtifactRequest {
    #[schemars(description = "Work item UUID to attach this artifact to")]
    work_item_id: Option<Uuid>,
    #[schemars(description = "Artifact kind: excalidraw or markdown")]
    kind: String,
    #[schemars(description = "Artifact title")]
    title: String,
    #[schemars(description = "Plain text body (for markdown artifacts)")]
    body_text: Option<String>,
    #[schemars(description = "JSON body (for excalidraw artifacts)")]
    body_json: Option<String>,
    #[schemars(description = "Parent artifact ID for versioning")]
    parent_artifact_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ListArtifactsRequest {
    #[schemars(description = "Filter by work item UUID")]
    work_item_id: Option<Uuid>,
    #[schemars(description = "Filter by kind: excalidraw or markdown")]
    kind: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetArtifactRequest {
    #[schemars(description = "Artifact UUID")]
    id: Uuid,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AcceptArtifactRequest {
    #[schemars(description = "Artifact UUID")]
    id: Uuid,
    #[schemars(description = "true to accept, false to un-accept")]
    accepted: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SharedContextRequest {
    #[schemars(description = "Work item UUID")]
    work_item_id: Uuid,
}

#[tool_router(router = artifact_tools_router, vis = "pub")]
impl McpServer {
    #[tool(
        name = "artifact.create",
        description = "Create a markdown or excalidraw artifact, optionally linked to a work item."
    )]
    async fn artifact_create(
        &self,
        Parameters(req): Parameters<CreateArtifactRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match artifact_db().await {
            Ok(db) => db,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        let id = Uuid::new_v4();
        let workspace_id = self.scoped_workspace_id();
        let created_by_kind = if workspace_id.is_some() {
            "workspace"
        } else {
            "human"
        };
        let result = sqlx::query!(
            r#"INSERT INTO artifacts (id, work_item_id, kind, title, body_text, body_json,
               parent_artifact_id, created_by_kind, created_by_workspace_id)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            id,
            req.work_item_id,
            req.kind,
            req.title,
            req.body_text,
            req.body_json,
            req.parent_artifact_id,
            created_by_kind,
            workspace_id
        )
        .execute(&db.pool)
        .await;
        match result {
            Ok(_) => match fetch_artifact(&db, id).await {
                Ok(a) => Self::success(&a),
                Err(e) => Ok(Self::tool_error(e)),
            },
            Err(e) => Ok(Self::tool_error(ToolError::new(
                "DB error",
                Some(e.to_string()),
            ))),
        }
    }

    #[tool(
        name = "artifact.list",
        description = "List artifacts, optionally filtered by work_item_id and/or kind."
    )]
    async fn artifact_list(
        &self,
        Parameters(req): Parameters<ListArtifactsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match artifact_db().await {
            Ok(db) => db,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        let rows = match (&req.work_item_id, &req.kind) {
            (Some(wi), Some(k)) => {
                sqlx::query_as!(
                    Artifact,
                    r#"SELECT id AS "id!: Uuid", work_item_id AS "work_item_id?: Uuid",
                       kind AS "kind!: ArtifactKind", title, body_text, body_json,
                       version AS "version!: i64", parent_artifact_id AS "parent_artifact_id?: Uuid",
                       created_by_kind, created_by_workspace_id AS "created_by_workspace_id?: Uuid",
                       created_by_execution_process_id AS "created_by_execution_process_id?: Uuid",
                       created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
                       accepted_at AS "accepted_at?: chrono::DateTime<chrono::Utc>"
                       FROM artifacts WHERE work_item_id = ?1 AND kind = ?2
                       ORDER BY created_at DESC"#,
                    wi, k
                )
                .fetch_all(&db.pool)
                .await
            }
            (Some(wi), None) => {
                sqlx::query_as!(
                    Artifact,
                    r#"SELECT id AS "id!: Uuid", work_item_id AS "work_item_id?: Uuid",
                       kind AS "kind!: ArtifactKind", title, body_text, body_json,
                       version AS "version!: i64", parent_artifact_id AS "parent_artifact_id?: Uuid",
                       created_by_kind, created_by_workspace_id AS "created_by_workspace_id?: Uuid",
                       created_by_execution_process_id AS "created_by_execution_process_id?: Uuid",
                       created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
                       accepted_at AS "accepted_at?: chrono::DateTime<chrono::Utc>"
                       FROM artifacts WHERE work_item_id = ?1
                       ORDER BY created_at DESC"#,
                    wi
                )
                .fetch_all(&db.pool)
                .await
            }
            (None, Some(k)) => {
                sqlx::query_as!(
                    Artifact,
                    r#"SELECT id AS "id!: Uuid", work_item_id AS "work_item_id?: Uuid",
                       kind AS "kind!: ArtifactKind", title, body_text, body_json,
                       version AS "version!: i64", parent_artifact_id AS "parent_artifact_id?: Uuid",
                       created_by_kind, created_by_workspace_id AS "created_by_workspace_id?: Uuid",
                       created_by_execution_process_id AS "created_by_execution_process_id?: Uuid",
                       created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
                       accepted_at AS "accepted_at?: chrono::DateTime<chrono::Utc>"
                       FROM artifacts WHERE kind = ?1
                       ORDER BY created_at DESC"#,
                    k
                )
                .fetch_all(&db.pool)
                .await
            }
            (None, None) => {
                sqlx::query_as!(
                    Artifact,
                    r#"SELECT id AS "id!: Uuid", work_item_id AS "work_item_id?: Uuid",
                       kind AS "kind!: ArtifactKind", title, body_text, body_json,
                       version AS "version!: i64", parent_artifact_id AS "parent_artifact_id?: Uuid",
                       created_by_kind, created_by_workspace_id AS "created_by_workspace_id?: Uuid",
                       created_by_execution_process_id AS "created_by_execution_process_id?: Uuid",
                       created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
                       accepted_at AS "accepted_at?: chrono::DateTime<chrono::Utc>"
                       FROM artifacts ORDER BY created_at DESC"#
                )
                .fetch_all(&db.pool)
                .await
            }
        };
        match rows {
            Ok(items) => Self::success(&items),
            Err(e) => Ok(Self::tool_error(ToolError::new(
                "DB error",
                Some(e.to_string()),
            ))),
        }
    }

    #[tool(name = "artifact.get", description = "Get a single artifact by ID.")]
    async fn artifact_get(
        &self,
        Parameters(req): Parameters<GetArtifactRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match artifact_db().await {
            Ok(db) => db,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        match fetch_artifact(&db, req.id).await {
            Ok(a) => Self::success(&a),
            Err(e) => Ok(Self::tool_error(e)),
        }
    }

    #[tool(
        name = "artifact.accept",
        description = "Accept or un-accept an artifact. Accepted artifacts appear in the work item's shared context."
    )]
    async fn artifact_accept(
        &self,
        Parameters(req): Parameters<AcceptArtifactRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match artifact_db().await {
            Ok(db) => db,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        let result = if req.accepted {
            let now_str = chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string();
            sqlx::query!(
                "UPDATE artifacts SET accepted_at = ?1 WHERE id = ?2",
                now_str,
                req.id
            )
            .execute(&db.pool)
            .await
        } else {
            sqlx::query!(
                "UPDATE artifacts SET accepted_at = NULL WHERE id = ?1",
                req.id
            )
            .execute(&db.pool)
            .await
        };
        match result {
            Ok(_) => match fetch_artifact(&db, req.id).await {
                Ok(a) => Self::success(&a),
                Err(e) => Ok(Self::tool_error(e)),
            },
            Err(e) => Ok(Self::tool_error(ToolError::new(
                "DB error",
                Some(e.to_string()),
            ))),
        }
    }

    #[tool(
        name = "artifact.shared_context",
        description = "Get the shared context for a work item: accepted artifacts + answered mail responses."
    )]
    async fn artifact_shared_context(
        &self,
        Parameters(req): Parameters<SharedContextRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match artifact_db().await {
            Ok(db) => db,
            Err(e) => return Ok(Self::tool_error(e)),
        };
        #[derive(serde::Serialize, sqlx::FromRow)]
        struct ContextItem {
            work_item_id: Uuid,
            context_type: String,
            context_id: Uuid,
            context_title: Option<String>,
            context_body: Option<String>,
            context_at: Option<String>,
        }
        let rows = sqlx::query_as!(
            ContextItem,
            r#"SELECT
               work_item_id AS "work_item_id!: Uuid",
               context_type AS "context_type!: String",
               context_id AS "context_id!: Uuid",
               context_title AS "context_title: String",
               context_body AS "context_body: String",
               context_at
               FROM work_item_shared_context
               WHERE work_item_id = ?1
               ORDER BY context_at DESC"#,
            req.work_item_id
        )
        .fetch_all(&db.pool)
        .await;
        match rows {
            Ok(items) => Self::success(&items),
            Err(e) => Ok(Self::tool_error(ToolError::new(
                "DB error",
                Some(e.to_string()),
            ))),
        }
    }
}

async fn artifact_db() -> Result<DBService, ToolError> {
    DBService::new()
        .await
        .map_err(|e| ToolError::new("Failed to open VK database", Some(e.to_string())))
}

async fn fetch_artifact(db: &DBService, id: Uuid) -> Result<Artifact, ToolError> {
    sqlx::query_as!(
        Artifact,
        r#"SELECT id AS "id!: Uuid", work_item_id AS "work_item_id?: Uuid",
           kind AS "kind!: ArtifactKind", title, body_text, body_json,
           version AS "version!: i64", parent_artifact_id AS "parent_artifact_id?: Uuid",
           created_by_kind, created_by_workspace_id AS "created_by_workspace_id?: Uuid",
           created_by_execution_process_id AS "created_by_execution_process_id?: Uuid",
           created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
           accepted_at AS "accepted_at?: chrono::DateTime<chrono::Utc>"
           FROM artifacts WHERE id = ?1"#,
        id
    )
    .fetch_one(&db.pool)
    .await
    .map_err(|e| ToolError::new("artifact not found", Some(e.to_string())))
}
