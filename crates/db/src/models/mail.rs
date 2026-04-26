use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
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

#[derive(Debug, Error)]
pub enum MailError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("mail resource not found")]
    NotFound,
    #[error("already_responded")]
    AlreadyResponded,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailSender {
    pub kind: MailSenderKind,
    pub workspace_id: Option<Uuid>,
    pub execution_process_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailTarget {
    pub kind: MailRecipientKind,
    pub workspace_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMailRequest {
    pub thread_id: Option<Uuid>,
    pub work_item_id: Option<Uuid>,
    pub kind: MailThreadKind,
    pub body: String,
    pub requires_response: bool,
    pub response_kind: Option<MailResponseKind>,
    pub response_options_json: Option<String>,
    pub sender: MailSender,
    pub target: MailTarget,
    pub idempotency_key: Option<String>,
    pub expires_in_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMailResponse {
    pub message_id: Uuid,
    pub thread_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyMailRequest {
    pub response_value_json: String,
    pub recipient_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailOkResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailSenderSummary {
    pub kind: MailSenderKind,
    pub workspace_id: Option<Uuid>,
    pub execution_process_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailThreadSummary {
    pub thread_id: Uuid,
    pub subject: String,
    pub kind: MailThreadKind,
    pub last_message_at: DateTime<Utc>,
    pub unread_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessageWithRecipients {
    pub message: MailMessage,
    pub recipients: Vec<MailRecipient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailThreadWithMessages {
    pub thread: MailThread,
    pub messages: Vec<MailMessageWithRecipients>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessageWithThread {
    pub thread: MailThread,
    pub message: MailMessage,
    pub recipients: Vec<MailRecipient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnreadMailItem {
    pub workspace_id: Option<Uuid>,
    pub recipient_id: Uuid,
    pub recipient_kind: MailRecipientKind,
    pub message_id: Uuid,
    pub thread_id: Uuid,
    pub subject: String,
    pub sender: MailSenderSummary,
    pub requires_response: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitingReplyItem {
    pub workspace_id: Uuid,
    pub message_id: Uuid,
    pub thread_id: Uuid,
    pub subject: String,
    pub requires_response: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ThreadSummaryRow {
    thread_id: Uuid,
    subject: String,
    kind: MailThreadKind,
    last_message_at: DateTime<Utc>,
    unread_count: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct UnreadMailRow {
    workspace_id: Option<Uuid>,
    recipient_id: Uuid,
    recipient_kind: MailRecipientKind,
    message_id: Uuid,
    thread_id: Uuid,
    subject: String,
    sender_kind: MailSenderKind,
    sender_workspace_id: Option<Uuid>,
    sender_execution_process_id: Option<Uuid>,
    requires_response: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct AwaitingReplyRow {
    workspace_id: Uuid,
    message_id: Uuid,
    thread_id: Uuid,
    subject: String,
    requires_response: bool,
    created_at: DateTime<Utc>,
}

impl From<ThreadSummaryRow> for MailThreadSummary {
    fn from(row: ThreadSummaryRow) -> Self {
        Self {
            thread_id: row.thread_id,
            subject: row.subject,
            kind: row.kind,
            last_message_at: row.last_message_at,
            unread_count: row.unread_count,
            created_at: row.created_at,
        }
    }
}

impl From<UnreadMailRow> for UnreadMailItem {
    fn from(row: UnreadMailRow) -> Self {
        Self {
            workspace_id: row.workspace_id,
            recipient_id: row.recipient_id,
            recipient_kind: row.recipient_kind,
            message_id: row.message_id,
            thread_id: row.thread_id,
            subject: row.subject,
            sender: MailSenderSummary {
                kind: row.sender_kind,
                workspace_id: row.sender_workspace_id,
                execution_process_id: row.sender_execution_process_id,
            },
            requires_response: row.requires_response,
            created_at: row.created_at,
        }
    }
}

impl From<AwaitingReplyRow> for AwaitingReplyItem {
    fn from(row: AwaitingReplyRow) -> Self {
        Self {
            workspace_id: row.workspace_id,
            message_id: row.message_id,
            thread_id: row.thread_id,
            subject: row.subject,
            requires_response: row.requires_response,
            created_at: row.created_at,
        }
    }
}

impl MailThread {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            MailThread,
            r#"SELECT
                id AS "id!: Uuid",
                work_item_id AS "work_item_id?: Uuid",
                subject,
                kind AS "kind!: MailThreadKind",
                created_at AS "created_at!: DateTime<Utc>",
                closed_at AS "closed_at?: DateTime<Utc>"
            FROM mail_threads
            WHERE id = ?"#,
            id
        )
        .fetch_optional(pool)
        .await
    }
}

impl MailMessage {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            MailMessage,
            r#"SELECT
                id AS "id!: Uuid",
                thread_id AS "thread_id!: Uuid",
                sender_kind AS "sender_kind!: MailSenderKind",
                sender_workspace_id AS "sender_workspace_id?: Uuid",
                sender_execution_process_id AS "sender_execution_process_id?: Uuid",
                body,
                requires_response AS "requires_response!: bool",
                response_kind AS "response_kind?: MailResponseKind",
                response_options_json,
                expires_at AS "expires_at?: DateTime<Utc>",
                idempotency_key,
                created_at AS "created_at!: DateTime<Utc>"
            FROM mail_messages
            WHERE id = ?"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn list_by_thread_id(
        pool: &SqlitePool,
        thread_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            MailMessage,
            r#"SELECT
                id AS "id!: Uuid",
                thread_id AS "thread_id!: Uuid",
                sender_kind AS "sender_kind!: MailSenderKind",
                sender_workspace_id AS "sender_workspace_id?: Uuid",
                sender_execution_process_id AS "sender_execution_process_id?: Uuid",
                body,
                requires_response AS "requires_response!: bool",
                response_kind AS "response_kind?: MailResponseKind",
                response_options_json,
                expires_at AS "expires_at?: DateTime<Utc>",
                idempotency_key,
                created_at AS "created_at!: DateTime<Utc>"
            FROM mail_messages
            WHERE thread_id = ?
            ORDER BY created_at ASC"#,
            thread_id
        )
        .fetch_all(pool)
        .await
    }
}

impl MailRecipient {
    pub async fn list_by_message_id(
        pool: &SqlitePool,
        message_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            MailRecipient,
            r#"SELECT
                id AS "id!: Uuid",
                message_id AS "message_id!: Uuid",
                recipient_kind AS "recipient_kind!: MailRecipientKind",
                recipient_workspace_id AS "recipient_workspace_id?: Uuid",
                read_at AS "read_at?: DateTime<Utc>",
                responded_at AS "responded_at?: DateTime<Utc>",
                response_value_json
            FROM mail_recipients
            WHERE message_id = ?
            ORDER BY id ASC"#,
            message_id
        )
        .fetch_all(pool)
        .await
    }
}

pub async fn send_mail(
    pool: &SqlitePool,
    request: SendMailRequest,
) -> Result<SendMailResponse, MailError> {
    validate_send_request(pool, &request).await?;

    if let Some(idempotency_key) = request.idempotency_key.as_deref()
        && let Some(existing) = find_message_by_idempotency_key(pool, idempotency_key).await?
    {
        return Ok(existing);
    }

    let expires_at = expires_at(request.expires_in_seconds)?;
    let thread_id = request.thread_id.unwrap_or_else(Uuid::new_v4);
    let message_id = Uuid::new_v4();
    let recipient_id = Uuid::new_v4();
    let subject = synthesize_subject(&request.body);
    let is_new_thread = request.thread_id.is_none();
    let work_item_id = request.work_item_id;
    let thread_kind: MailThreadKind = request.kind;
    let sender_kind: MailSenderKind = request.sender.kind;
    let sender_workspace_id = request.sender.workspace_id;
    let sender_execution_process_id = request.sender.execution_process_id;
    let body = request.body;
    let requires_response = request.requires_response;
    let response_kind: Option<MailResponseKind> = request.response_kind;
    let response_options_json = request.response_options_json;
    let target_kind: MailRecipientKind = request.target.kind;
    let target_workspace_id = request.target.workspace_id;
    let idempotency_key = request.idempotency_key;

    let mut tx = pool.begin().await?;

    if is_new_thread {
        sqlx::query!(
            r#"INSERT INTO mail_threads (id, work_item_id, subject, kind)
               VALUES (?1, ?2, ?3, ?4)"#,
            thread_id,
            work_item_id,
            subject,
            thread_kind
        )
        .execute(&mut *tx)
        .await?;
    }

    let message_insert = sqlx::query!(
        r#"INSERT INTO mail_messages (
               id,
               thread_id,
               sender_kind,
               sender_workspace_id,
               sender_execution_process_id,
               body,
               requires_response,
               response_kind,
               response_options_json,
               expires_at,
               idempotency_key
           )
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"#,
        message_id,
        thread_id,
        sender_kind,
        sender_workspace_id,
        sender_execution_process_id,
        body,
        requires_response,
        response_kind,
        response_options_json,
        expires_at,
        idempotency_key
    )
    .execute(&mut *tx)
    .await;

    if let Err(err) = message_insert {
        // The pre-check at line ~383 narrows but does not eliminate the
        // idempotency-key race: two concurrent sends with the same key can
        // both pass the SELECT, then exactly one INSERT wins and the other
        // hits the partial unique index. Recover gracefully by rolling back
        // and returning the existing row, so callers still get an idempotent
        // response (matching the design doc's fix #6 contract).
        if let (Some(db_err), Some(key)) = (err.as_database_error(), idempotency_key.as_deref())
            && db_err.is_unique_violation()
        {
            tx.rollback().await?;
            if let Some(existing) = find_message_by_idempotency_key(pool, key).await? {
                return Ok(existing);
            }
        }
        return Err(err.into());
    }

    sqlx::query!(
        r#"INSERT INTO mail_recipients (
               id,
               message_id,
               recipient_kind,
               recipient_workspace_id
           )
           VALUES (?1, ?2, ?3, ?4)"#,
        recipient_id,
        message_id,
        target_kind,
        target_workspace_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(SendMailResponse {
        message_id,
        thread_id,
    })
}

pub async fn list_threads_for_workspace(
    pool: &SqlitePool,
    workspace_id: Uuid,
    limit: i64,
) -> Result<Vec<MailThreadSummary>, MailError> {
    let rows = sqlx::query_as!(
        ThreadSummaryRow,
        r#"SELECT
            t.id AS "thread_id!: Uuid",
            t.subject AS "subject!",
            t.kind AS "kind!: MailThreadKind",
            MAX(m.created_at) AS "last_message_at!: DateTime<Utc>",
            COALESCE(SUM(
                CASE
                    WHEN r.recipient_kind = 'workspace'
                     AND r.recipient_workspace_id = ?1
                     AND r.read_at IS NULL
                    THEN 1
                    ELSE 0
                END
            ), 0) AS "unread_count!: i64",
            t.created_at AS "created_at!: DateTime<Utc>"
        FROM mail_threads t
        JOIN mail_messages m ON m.thread_id = t.id
        LEFT JOIN mail_recipients r ON r.message_id = m.id
        WHERE m.sender_workspace_id = ?1
           OR EXISTS (
               SELECT 1
               FROM mail_recipients rx
               JOIN mail_messages mx ON mx.id = rx.message_id
               WHERE mx.thread_id = t.id
                 AND rx.recipient_kind = 'workspace'
                 AND rx.recipient_workspace_id = ?1
           )
        GROUP BY t.id
        ORDER BY MAX(m.created_at) DESC
        LIMIT ?2"#,
        workspace_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_thread_with_messages(
    pool: &SqlitePool,
    thread_id: Uuid,
) -> Result<MailThreadWithMessages, MailError> {
    let thread = MailThread::find_by_id(pool, thread_id)
        .await?
        .ok_or(MailError::NotFound)?;
    let messages = MailMessage::list_by_thread_id(pool, thread_id)
        .await?
        .into_iter();
    let mut messages_with_recipients = Vec::new();

    for message in messages {
        let recipients = MailRecipient::list_by_message_id(pool, message.id).await?;
        messages_with_recipients.push(MailMessageWithRecipients {
            message,
            recipients,
        });
    }

    Ok(MailThreadWithMessages {
        thread,
        messages: messages_with_recipients,
    })
}

pub async fn get_message_with_recipients(
    pool: &SqlitePool,
    message_id: Uuid,
) -> Result<MailMessageWithRecipients, MailError> {
    let message = MailMessage::find_by_id(pool, message_id)
        .await?
        .ok_or(MailError::NotFound)?;
    let recipients = MailRecipient::list_by_message_id(pool, message_id).await?;

    Ok(MailMessageWithRecipients {
        message,
        recipients,
    })
}

pub async fn get_message_with_thread(
    pool: &SqlitePool,
    message_id: Uuid,
) -> Result<MailMessageWithThread, MailError> {
    let message = MailMessage::find_by_id(pool, message_id)
        .await?
        .ok_or(MailError::NotFound)?;
    let thread = MailThread::find_by_id(pool, message.thread_id)
        .await?
        .ok_or(MailError::NotFound)?;
    let recipients = MailRecipient::list_by_message_id(pool, message_id).await?;

    Ok(MailMessageWithThread {
        thread,
        message,
        recipients,
    })
}

pub async fn reply_to_message(
    pool: &SqlitePool,
    message_id: Uuid,
    recipient_id: Uuid,
    response_value_json: String,
) -> Result<MailOkResponse, MailError> {
    let affected = sqlx::query!(
        "UPDATE mail_recipients SET responded_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'),
               response_value_json = ?1
        WHERE id = ?2 AND message_id = ?3 AND responded_at IS NULL",
        response_value_json,
        recipient_id,
        message_id
    )
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(MailError::AlreadyResponded);
    }

    Ok(MailOkResponse { ok: true })
}

pub async fn mark_message_read_for_recipient(
    pool: &SqlitePool,
    message_id: Uuid,
    recipient_kind: MailRecipientKind,
    workspace_id: Option<Uuid>,
) -> Result<(), MailError> {
    match recipient_kind {
        MailRecipientKind::Workspace => {
            let workspace_id = workspace_id.ok_or_else(|| {
                MailError::InvalidRequest("workspace_id is required for workspace recipient".into())
            })?;
            sqlx::query!(
                "UPDATE mail_recipients
                    SET read_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                  WHERE message_id = ?1
                    AND recipient_kind = 'workspace'
                    AND recipient_workspace_id = ?2
                    AND read_at IS NULL",
                message_id,
                workspace_id
            )
            .execute(pool)
            .await?;
        }
        MailRecipientKind::Human => {
            sqlx::query!(
                "UPDATE mail_recipients
                    SET read_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                  WHERE message_id = ?1
                    AND recipient_kind = 'human'
                    AND read_at IS NULL",
                message_id
            )
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

pub async fn list_workspace_unread(
    pool: &SqlitePool,
    workspace_id: Uuid,
) -> Result<Vec<UnreadMailItem>, MailError> {
    let rows = sqlx::query_as!(
        UnreadMailRow,
        r#"SELECT
            v.workspace_id AS "workspace_id?: Uuid",
            v.recipient_id AS "recipient_id!: Uuid",
            'workspace' AS "recipient_kind!: MailRecipientKind",
            v.message_id AS "message_id!: Uuid",
            v.thread_id AS "thread_id!: Uuid",
            t.subject,
            m.sender_kind AS "sender_kind!: MailSenderKind",
            m.sender_workspace_id AS "sender_workspace_id?: Uuid",
            m.sender_execution_process_id AS "sender_execution_process_id?: Uuid",
            m.requires_response AS "requires_response!: bool",
            v.created_at AS "created_at!: DateTime<Utc>"
        FROM workspace_inbox_unread v
        JOIN mail_messages m ON m.id = v.message_id
        JOIN mail_threads t ON t.id = v.thread_id
        WHERE v.workspace_id = ?1
        ORDER BY v.created_at DESC"#,
        workspace_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn list_human_unread(pool: &SqlitePool) -> Result<Vec<UnreadMailItem>, MailError> {
    let rows = sqlx::query_as!(
        UnreadMailRow,
        r#"SELECT
            NULL AS "workspace_id?: Uuid",
            r.id AS "recipient_id!: Uuid",
            r.recipient_kind AS "recipient_kind!: MailRecipientKind",
            r.message_id AS "message_id!: Uuid",
            m.thread_id AS "thread_id!: Uuid",
            t.subject,
            m.sender_kind AS "sender_kind!: MailSenderKind",
            m.sender_workspace_id AS "sender_workspace_id?: Uuid",
            m.sender_execution_process_id AS "sender_execution_process_id?: Uuid",
            m.requires_response AS "requires_response!: bool",
            m.created_at AS "created_at!: DateTime<Utc>"
        FROM mail_recipients r
        JOIN mail_messages m ON m.id = r.message_id
        JOIN mail_threads t ON t.id = m.thread_id
        WHERE r.recipient_kind = 'human'
          AND r.read_at IS NULL
        ORDER BY m.created_at DESC"#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn list_workspace_awaiting_reply(
    pool: &SqlitePool,
    workspace_id: Uuid,
) -> Result<Vec<AwaitingReplyItem>, MailError> {
    let rows = sqlx::query_as!(
        AwaitingReplyRow,
        r#"SELECT
            v.workspace_id AS "workspace_id!: Uuid",
            v.message_id AS "message_id!: Uuid",
            v.thread_id AS "thread_id!: Uuid",
            t.subject,
            m.requires_response AS "requires_response!: bool",
            v.created_at AS "created_at!: DateTime<Utc>"
        FROM workspace_awaiting_reply v
        JOIN mail_messages m ON m.id = v.message_id
        JOIN mail_threads t ON t.id = v.thread_id
        WHERE v.workspace_id = ?1
        ORDER BY v.created_at DESC"#,
        workspace_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn validate_send_request(
    pool: &SqlitePool,
    request: &SendMailRequest,
) -> Result<(), MailError> {
    if request.kind != MailThreadKind::AgentHuman {
        return Err(MailError::InvalidRequest(
            "Phase 4a only supports kind agent_human".into(),
        ));
    }

    if request.body.trim().is_empty() {
        return Err(MailError::InvalidRequest("body must not be empty".into()));
    }

    if let Some(response_options_json) = request.response_options_json.as_deref() {
        serde_json::from_str::<serde_json::Value>(response_options_json).map_err(|error| {
            MailError::InvalidRequest(format!("response_options_json must be valid JSON: {error}"))
        })?;
    }

    validate_sender(pool, &request.sender).await?;
    validate_target(pool, &request.target).await?;

    if let Some(thread_id) = request.thread_id {
        let thread = MailThread::find_by_id(pool, thread_id)
            .await?
            .ok_or(MailError::NotFound)?;
        if thread.kind != MailThreadKind::AgentHuman {
            return Err(MailError::InvalidRequest(
                "Phase 4a only supports appending to agent_human threads".into(),
            ));
        }
    }

    Ok(())
}

async fn validate_sender(pool: &SqlitePool, sender: &MailSender) -> Result<(), MailError> {
    match sender.kind {
        MailSenderKind::Workspace => {
            let workspace_id = sender.workspace_id.ok_or_else(|| {
                MailError::InvalidRequest("sender.workspace_id is required".into())
            })?;
            if !workspace_exists(pool, workspace_id).await? {
                return Err(MailError::NotFound);
            }
        }
        MailSenderKind::Human => {
            if sender.workspace_id.is_some() || sender.execution_process_id.is_some() {
                return Err(MailError::InvalidRequest(
                    "human sender must not include workspace_id or execution_process_id".into(),
                ));
            }
        }
    }

    Ok(())
}

async fn validate_target(pool: &SqlitePool, target: &MailTarget) -> Result<(), MailError> {
    match target.kind {
        MailRecipientKind::Workspace => {
            let workspace_id = target.workspace_id.ok_or_else(|| {
                MailError::InvalidRequest("target.workspace_id is required".into())
            })?;
            if !workspace_exists(pool, workspace_id).await? {
                return Err(MailError::NotFound);
            }
        }
        MailRecipientKind::Human => {
            if target.workspace_id.is_some() {
                return Err(MailError::InvalidRequest(
                    "human target must not include workspace_id".into(),
                ));
            }
        }
    }

    Ok(())
}

async fn workspace_exists(pool: &SqlitePool, workspace_id: Uuid) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = ?1) AS "exists!: bool""#,
        workspace_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row.exists)
}

async fn find_message_by_idempotency_key(
    pool: &SqlitePool,
    idempotency_key: &str,
) -> Result<Option<SendMailResponse>, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT
            id AS "message_id!: Uuid",
            thread_id AS "thread_id!: Uuid"
        FROM mail_messages
        WHERE idempotency_key = ?1"#,
        idempotency_key
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| SendMailResponse {
        message_id: row.message_id,
        thread_id: row.thread_id,
    }))
}

fn expires_at(expires_in_seconds: Option<u64>) -> Result<Option<DateTime<Utc>>, MailError> {
    let Some(seconds) = expires_in_seconds else {
        return Ok(None);
    };
    let seconds = i64::try_from(seconds).map_err(|_| {
        MailError::InvalidRequest("expires_in_seconds is too large to represent".into())
    })?;
    Utc::now()
        .checked_add_signed(Duration::seconds(seconds))
        .map(Some)
        .ok_or_else(|| MailError::InvalidRequest("expires_in_seconds is out of range".into()))
}

fn synthesize_subject(body: &str) -> String {
    body.trim().chars().take(80).collect()
}
