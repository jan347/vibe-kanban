use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct RepoBundle {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    /// JSON array of repo UUIDs (TEXT in SQLite). Parse with serde_json on read.
    pub repo_ids_json: String,
    /// JSON map: repo_id -> branch override. Optional.
    pub default_branch_overrides_json: Option<String>,
    pub default_preset_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateRepoBundle {
    pub name: String,
    pub description: Option<String>,
    pub repo_ids: Vec<Uuid>,
    pub default_branch_overrides: Option<std::collections::HashMap<Uuid, String>>,
    pub default_preset_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateRepoBundle {
    pub name: Option<String>,
    pub description: Option<String>,
    pub repo_ids: Option<Vec<Uuid>>,
    pub default_branch_overrides: Option<std::collections::HashMap<Uuid, String>>,
    pub default_preset_id: Option<Uuid>,
}
