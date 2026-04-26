use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "mail_thread_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MailThreadKind {
    AgentHuman,
    AgentAgent,
    Broadcast,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "mail_sender_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MailSenderKind {
    Workspace,
    Human,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "mail_response_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MailResponseKind {
    Options,
    FreeText,
    File,
    Approval,
    None,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "mail_recipient_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MailRecipientKind {
    Workspace,
    Human,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MailThread {
    pub id: Uuid,
    pub work_item_id: Option<Uuid>,
    pub subject: String,
    pub kind: MailThreadKind,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MailMessage {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub sender_kind: MailSenderKind,
    pub sender_workspace_id: Option<Uuid>,
    pub sender_execution_process_id: Option<Uuid>,
    pub body: String,
    pub requires_response: bool,
    pub response_kind: Option<MailResponseKind>,
    pub response_options_json: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub idempotency_key: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MailRecipient {
    pub id: Uuid,
    pub message_id: Uuid,
    pub recipient_kind: MailRecipientKind,
    pub recipient_workspace_id: Option<Uuid>,
    pub read_at: Option<DateTime<Utc>>,
    pub responded_at: Option<DateTime<Utc>>,
    pub response_value_json: Option<String>,
}
