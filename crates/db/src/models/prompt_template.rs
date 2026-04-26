use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "prompt_template_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PromptTemplateRole {
    Implement,
    Investigate,
    Review,
    Qa,
    DesignPolish,
    Docs,
    Security,
    Other,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct PromptTemplate {
    pub id: Uuid,
    pub name: String,
    pub role: PromptTemplateRole,
    pub description: Option<String>,
    pub body_text: String,
    pub preset_id: Option<Uuid>,
    pub bundle_id: Option<Uuid>,
    pub tags_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreatePromptTemplate {
    pub name: String,
    pub role: PromptTemplateRole,
    pub description: Option<String>,
    pub body_text: String,
    pub preset_id: Option<Uuid>,
    pub bundle_id: Option<Uuid>,
    pub tags_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdatePromptTemplate {
    pub name: Option<String>,
    pub role: Option<PromptTemplateRole>,
    pub description: Option<String>,
    pub body_text: Option<String>,
    pub preset_id: Option<Uuid>,
    pub bundle_id: Option<Uuid>,
    pub tags_json: Option<String>,
}
