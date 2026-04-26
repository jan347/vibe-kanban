use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "work_item_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorkItemStatus {
    Open,
    InProgress,
    Blocked,
    Done,
    Archived,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct WorkItem {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: WorkItemStatus,
    pub priority: i64,
    pub tags_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct WorkItemRun {
    pub work_item_id: Uuid,
    pub workspace_id: Uuid,
    pub role: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateWorkItem {
    pub title: String,
    pub description: Option<String>,
    pub status: Option<WorkItemStatus>,
    pub priority: Option<i64>,
    pub tags_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateWorkItem {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<WorkItemStatus>,
    pub priority: Option<i64>,
    pub tags_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct LinkWorkspaceToWorkItem {
    pub workspace_id: Uuid,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct WorkItemWithLinks {
    pub work_item: WorkItem,
    pub linked_runs: Vec<WorkItemRun>,
}
