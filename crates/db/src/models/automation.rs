use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AutomationRule {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub work_item_id: Uuid,
    pub name: String,
    pub trigger_kind: String,
    pub trigger_config: String,
    pub prompt_template_id: Option<Uuid>,
    pub model_preset_id: Option<Uuid>,
    pub prompt_override: Option<String>,
    pub enabled: bool,
    pub last_fired_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateAutomationRule {
    pub workspace_id: Uuid,
    pub work_item_id: Uuid,
    pub name: String,
    pub trigger_kind: String,
    pub trigger_config: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub model_preset_id: Option<Uuid>,
    pub prompt_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateAutomationRule {
    pub name: Option<String>,
    pub trigger_kind: Option<String>,
    pub trigger_config: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub model_preset_id: Option<Uuid>,
    pub prompt_override: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FireAutomationResult {
    Approved {
        rule_id: Uuid,
        dispatch_id: Uuid,
        fired_at: DateTime<Utc>,
    },
    Blocked {
        rule_id: Uuid,
        decision: crate::models::safety::AutoApprovalDecision,
    },
}
