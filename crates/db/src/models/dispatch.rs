use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "dispatch_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DispatchStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct DispatchLogEntry {
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub workspace_id: Uuid,
    pub session_id: Option<Uuid>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_text: String,
    pub model_preset_id: Option<Uuid>,
    pub status: DispatchStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateDispatch {
    pub work_item_id: Uuid,
    pub workspace_id: Uuid,
    pub prompt_text: String,
    pub prompt_template_id: Option<Uuid>,
    pub model_preset_id: Option<Uuid>,
}
