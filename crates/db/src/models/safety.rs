use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct SafetyConfig {
    pub id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub scope: String,
    pub require_human_approval: bool,
    pub max_concurrent_dispatch: i64,
    pub max_daily_dispatch: i64,
    pub cooldown_seconds: i64,
    pub auto_approval_enabled: bool,
    pub auto_approval_policy: Option<String>,
    pub auto_approval_model_preset_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateSafetyConfig {
    pub require_human_approval: Option<bool>,
    pub max_concurrent_dispatch: Option<i64>,
    pub max_daily_dispatch: Option<i64>,
    pub cooldown_seconds: Option<i64>,
    pub auto_approval_enabled: Option<bool>,
    pub auto_approval_policy: Option<String>,
    pub auto_approval_model_preset_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SafetyCheckResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub active_dispatches: i64,
    pub daily_dispatches: i64,
    pub require_human_approval: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AutoApprovalLogEntry {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub action_kind: String,
    pub action_summary: String,
    pub decision: String,
    pub reasoning: Option<String>,
    pub decided_by: String,
    pub decided_at: DateTime<Utc>,
    pub resolved_decision: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub approval_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ResolveAutoApprovalRequest {
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AutoApprovalRequest {
    pub workspace_id: Uuid,
    pub action_kind: String,
    pub action_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AutoApprovalDecision {
    pub approved: bool,
    pub decision: String,
    pub reasoning: String,
    pub decided_by: String,
}
