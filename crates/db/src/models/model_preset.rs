use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "model_preset_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ModelPresetRole {
    Planner,
    Implementer,
    Reviewer,
    Qa,
    Diagrammer,
    Summarizer,
    Other,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "model_preset_executor", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelPresetExecutor {
    ClaudeCode,
    Codex,
    QwenCode,
    Opencode,
    Gemini,
    CursorAgent,
    Amp,
    Droid,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ModelPreset {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub role: ModelPresetRole,
    pub executor: ModelPresetExecutor,
    pub model_id: String,
    pub permission_mode: Option<String>,
    pub reasoning_effort: Option<String>,
    pub env_vars_json: Option<String>,
    pub labels_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateModelPreset {
    pub name: String,
    pub description: Option<String>,
    pub role: ModelPresetRole,
    pub executor: ModelPresetExecutor,
    pub model_id: String,
    pub permission_mode: Option<String>,
    pub reasoning_effort: Option<String>,
    pub env_vars_json: Option<String>,
    pub labels_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateModelPreset {
    pub name: Option<String>,
    pub description: Option<String>,
    pub role: Option<ModelPresetRole>,
    pub executor: Option<ModelPresetExecutor>,
    pub model_id: Option<String>,
    pub permission_mode: Option<String>,
    pub reasoning_effort: Option<String>,
    pub env_vars_json: Option<String>,
    pub labels_json: Option<String>,
}
