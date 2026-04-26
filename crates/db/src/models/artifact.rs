use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "artifact_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Excalidraw,
    Markdown,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Artifact {
    pub id: Uuid,
    pub work_item_id: Option<Uuid>,
    pub kind: ArtifactKind,
    pub title: String,
    pub body_text: Option<String>,
    pub body_json: Option<String>,
    pub version: i64,
    pub parent_artifact_id: Option<Uuid>,
    pub created_by_kind: String,
    pub created_by_workspace_id: Option<Uuid>,
    pub created_by_execution_process_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateArtifact {
    pub work_item_id: Option<Uuid>,
    pub kind: ArtifactKind,
    pub title: String,
    pub body_text: Option<String>,
    pub body_json: Option<String>,
    pub parent_artifact_id: Option<Uuid>,
    pub created_by_kind: String,
    pub created_by_workspace_id: Option<Uuid>,
    pub created_by_execution_process_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AcceptArtifactRequest {
    pub accepted: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct SharedContextItem {
    pub work_item_id: Uuid,
    pub context_type: String,
    pub context_id: Uuid,
    pub context_title: Option<String>,
    pub context_body: Option<String>,
    pub context_at: Option<DateTime<Utc>>,
}
