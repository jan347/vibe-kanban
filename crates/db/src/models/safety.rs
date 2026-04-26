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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateSafetyConfig {
    pub require_human_approval: Option<bool>,
    pub max_concurrent_dispatch: Option<i64>,
    pub max_daily_dispatch: Option<i64>,
    pub cooldown_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SafetyCheckResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub active_dispatches: i64,
    pub daily_dispatches: i64,
    pub require_human_approval: bool,
}
