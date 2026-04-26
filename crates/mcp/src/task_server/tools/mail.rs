use db::{
    DBService,
    models::mail::{
        MailError, MailRecipientKind, MailResponseKind, MailSender, MailSenderKind, MailTarget,
        MailThreadKind, ReplyMailRequest as DbReplyMailRequest,
        SendMailRequest as DbSendMailRequest, get_message_with_thread, list_human_unread,
        list_workspace_unread, mark_message_read_for_recipient, reply_to_message, send_mail,
    },
};
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool,
    tool_router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::{McpServer, ToolError};

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ToolMailThreadKind {
    AgentHuman,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ToolMailSenderKind {
    Workspace,
    Human,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ToolMailRecipientKind {
    Workspace,
    Human,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ToolMailResponseKind {
    Options,
    FreeText,
    File,
    Approval,
    None,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ToolMailSender {
    kind: ToolMailSenderKind,
    workspace_id: Option<Uuid>,
    execution_process_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ToolMailTarget {
    kind: ToolMailRecipientKind,
    workspace_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SendMailRequest {
    thread_id: Option<Uuid>,
    work_item_id: Option<Uuid>,
    kind: ToolMailThreadKind,
    body: String,
    requires_response: bool,
    response_kind: Option<ToolMailResponseKind>,
    response_options_json: Option<String>,
    sender: ToolMailSender,
    target: ToolMailTarget,
    idempotency_key: Option<String>,
    expires_in_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ListUnreadMailRequest {
    as_workspace_id: Option<Uuid>,
    as_human: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ReadMailRequest {
    message_id: Uuid,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ReplyMailRequest {
    message_id: Uuid,
    recipient_id: Uuid,
    response_value_json: String,
}

#[tool_router(router = mail_tools_router, vis = "pub")]
impl McpServer {
    #[tool(name = "mail.send", description = "Send an agent-human mail message.")]
    async fn mail_send(
        &self,
        Parameters(request): Parameters<SendMailRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let request = match self.normalize_send_request(request) {
            Ok(request) => request,
            Err(error) => return Ok(Self::tool_error(error)),
        };
        let db = match Self::mail_db().await {
            Ok(db) => db,
            Err(error) => return Ok(Self::tool_error(error)),
        };

        match send_mail(&db.pool, request).await {
            Ok(response) => Self::success(&response),
            Err(error) => Ok(Self::tool_error(mail_tool_error(error))),
        }
    }

    #[tool(
        name = "mail.list_unread",
        description = "List unread mail for a workspace, or human unread mail when as_human is true."
    )]
    async fn mail_list_unread(
        &self,
        Parameters(request): Parameters<ListUnreadMailRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match Self::mail_db().await {
            Ok(db) => db,
            Err(error) => return Ok(Self::tool_error(error)),
        };

        let workspace_id = if request.as_human.unwrap_or(false) {
            None
        } else {
            request
                .as_workspace_id
                .or_else(|| self.scoped_workspace_id())
        };

        let messages = if let Some(workspace_id) = workspace_id {
            if let Err(error) = self.scope_allows_workspace(workspace_id) {
                return Ok(Self::tool_error(error));
            }
            list_workspace_unread(&db.pool, workspace_id).await
        } else {
            list_human_unread(&db.pool).await
        };

        match messages {
            Ok(messages) => Self::success(&messages),
            Err(error) => Ok(Self::tool_error(mail_tool_error(error))),
        }
    }

    #[tool(
        name = "mail.read",
        description = "Read a mail message with thread context and mark it read for the caller."
    )]
    async fn mail_read(
        &self,
        Parameters(ReadMailRequest { message_id }): Parameters<ReadMailRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match Self::mail_db().await {
            Ok(db) => db,
            Err(error) => return Ok(Self::tool_error(error)),
        };

        let mark_read_result = if let Some(workspace_id) = self.scoped_workspace_id() {
            mark_message_read_for_recipient(
                &db.pool,
                message_id,
                MailRecipientKind::Workspace,
                Some(workspace_id),
            )
            .await
        } else {
            mark_message_read_for_recipient(&db.pool, message_id, MailRecipientKind::Human, None)
                .await
        };

        if let Err(error) = mark_read_result {
            return Ok(Self::tool_error(mail_tool_error(error)));
        }

        match get_message_with_thread(&db.pool, message_id).await {
            Ok(message) => Self::success(&message),
            Err(error) => Ok(Self::tool_error(mail_tool_error(error))),
        }
    }

    #[tool(
        name = "mail.reply",
        description = "Reply to a mail message recipient once."
    )]
    async fn mail_reply(
        &self,
        Parameters(request): Parameters<ReplyMailRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = match Self::mail_db().await {
            Ok(db) => db,
            Err(error) => return Ok(Self::tool_error(error)),
        };

        let DbReplyMailRequest {
            recipient_id,
            response_value_json,
        } = DbReplyMailRequest {
            recipient_id: request.recipient_id,
            response_value_json: request.response_value_json,
        };

        match reply_to_message(
            &db.pool,
            request.message_id,
            recipient_id,
            response_value_json,
        )
        .await
        {
            Ok(response) => Self::success(&response),
            Err(error) => Ok(Self::tool_error(mail_tool_error(error))),
        }
    }
}

impl McpServer {
    async fn mail_db() -> Result<DBService, ToolError> {
        DBService::new()
            .await
            .map_err(|error| ToolError::new("Failed to open VK database", Some(error.to_string())))
    }

    fn normalize_send_request(
        &self,
        request: SendMailRequest,
    ) -> Result<DbSendMailRequest, ToolError> {
        let sender_workspace_id =
            request
                .sender
                .workspace_id
                .or_else(|| match request.sender.kind {
                    ToolMailSenderKind::Workspace => self.scoped_workspace_id(),
                    ToolMailSenderKind::Human => None,
                });

        if let Some(workspace_id) = sender_workspace_id {
            self.scope_allows_workspace(workspace_id)?;
        }
        if let Some(workspace_id) = request.target.workspace_id {
            self.scope_allows_workspace(workspace_id)?;
        }

        Ok(DbSendMailRequest {
            thread_id: request.thread_id,
            work_item_id: request.work_item_id,
            kind: request.kind.into(),
            body: request.body,
            requires_response: request.requires_response,
            response_kind: request.response_kind.map(Into::into),
            response_options_json: request.response_options_json,
            sender: MailSender {
                kind: request.sender.kind.into(),
                workspace_id: sender_workspace_id,
                execution_process_id: request.sender.execution_process_id,
            },
            target: MailTarget {
                kind: request.target.kind.into(),
                workspace_id: request.target.workspace_id,
            },
            idempotency_key: request.idempotency_key,
            expires_in_seconds: request.expires_in_seconds,
        })
    }
}

impl From<ToolMailThreadKind> for MailThreadKind {
    fn from(value: ToolMailThreadKind) -> Self {
        match value {
            ToolMailThreadKind::AgentHuman => MailThreadKind::AgentHuman,
        }
    }
}

impl From<ToolMailSenderKind> for MailSenderKind {
    fn from(value: ToolMailSenderKind) -> Self {
        match value {
            ToolMailSenderKind::Workspace => MailSenderKind::Workspace,
            ToolMailSenderKind::Human => MailSenderKind::Human,
        }
    }
}

impl From<ToolMailRecipientKind> for MailRecipientKind {
    fn from(value: ToolMailRecipientKind) -> Self {
        match value {
            ToolMailRecipientKind::Workspace => MailRecipientKind::Workspace,
            ToolMailRecipientKind::Human => MailRecipientKind::Human,
        }
    }
}

impl From<ToolMailResponseKind> for MailResponseKind {
    fn from(value: ToolMailResponseKind) -> Self {
        match value {
            ToolMailResponseKind::Options => MailResponseKind::Options,
            ToolMailResponseKind::FreeText => MailResponseKind::FreeText,
            ToolMailResponseKind::File => MailResponseKind::File,
            ToolMailResponseKind::Approval => MailResponseKind::Approval,
            ToolMailResponseKind::None => MailResponseKind::None,
        }
    }
}

fn mail_tool_error(error: MailError) -> ToolError {
    match error {
        MailError::AlreadyResponded => ToolError::message("already_responded"),
        MailError::NotFound => ToolError::message("mail resource not found"),
        MailError::InvalidRequest(message) => ToolError::new("invalid mail request", Some(message)),
        MailError::Database(error) => {
            ToolError::new("mail database error", Some(error.to_string()))
        }
    }
}
