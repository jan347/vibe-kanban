use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::mail::{
    AwaitingReplyItem, BroadcastMailRequest, BroadcastMailResponse, CreateMailAttachment,
    MailAttachment, MailError, MailMessageWithRecipients, MailOkResponse, MailThreadSummary,
    MailThreadWithMessages, ReplyMailRequest, SendMailRequest, SendMailResponse, UnreadMailItem,
    attach_blob_to_message, broadcast_mail, get_message_with_recipients,
    get_thread_with_messages, list_attachments_for_message, list_threads_for_workspace,
    list_workspace_awaiting_reply, list_workspace_unread, reply_to_message, send_mail,
};
use deployment::Deployment;
use serde::Deserialize;
use serde_json::json;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::DeploymentImpl;

type MailRouteResult<T> =
    Result<ResponseJson<ApiResponse<T>>, (StatusCode, ResponseJson<serde_json::Value>)>;

#[derive(Debug, Deserialize)]
struct ListThreadsQuery {
    workspace_id: Uuid,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceQuery {
    workspace_id: Uuid,
}

async fn send(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<SendMailRequest>,
) -> MailRouteResult<SendMailResponse> {
    send_mail(&deployment.db().pool, payload)
        .await
        .map(|response| ResponseJson(ApiResponse::success(response)))
        .map_err(mail_error_response)
}

async fn broadcast(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<BroadcastMailRequest>,
) -> MailRouteResult<BroadcastMailResponse> {
    broadcast_mail(&deployment.db().pool, payload)
        .await
        .map(|response| ResponseJson(ApiResponse::success(response)))
        .map_err(mail_error_response)
}

async fn list_threads(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListThreadsQuery>,
) -> MailRouteResult<Vec<MailThreadSummary>> {
    let limit = query.limit.unwrap_or(50);
    if !(1..=500).contains(&limit) {
        return Err((
            StatusCode::BAD_REQUEST,
            ResponseJson(json!({ "error": "invalid_limit" })),
        ));
    }

    list_threads_for_workspace(&deployment.db().pool, query.workspace_id, limit)
        .await
        .map(|threads| ResponseJson(ApiResponse::success(threads)))
        .map_err(mail_error_response)
}

async fn get_thread(
    State(deployment): State<DeploymentImpl>,
    Path(thread_id): Path<Uuid>,
) -> MailRouteResult<MailThreadWithMessages> {
    get_thread_with_messages(&deployment.db().pool, thread_id)
        .await
        .map(|thread| ResponseJson(ApiResponse::success(thread)))
        .map_err(mail_error_response)
}

async fn get_message(
    State(deployment): State<DeploymentImpl>,
    Path(message_id): Path<Uuid>,
) -> MailRouteResult<MailMessageWithRecipients> {
    get_message_with_recipients(&deployment.db().pool, message_id)
        .await
        .map(|message| ResponseJson(ApiResponse::success(message)))
        .map_err(mail_error_response)
}

async fn reply(
    State(deployment): State<DeploymentImpl>,
    Path(message_id): Path<Uuid>,
    ResponseJson(payload): ResponseJson<ReplyMailRequest>,
) -> MailRouteResult<MailOkResponse> {
    reply_to_message(
        &deployment.db().pool,
        message_id,
        payload.recipient_id,
        payload.response_value_json,
    )
    .await
    .map(|response| ResponseJson(ApiResponse::success(response)))
    .map_err(mail_error_response)
}

async fn list_unread(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<WorkspaceQuery>,
) -> MailRouteResult<Vec<UnreadMailItem>> {
    list_workspace_unread(&deployment.db().pool, query.workspace_id)
        .await
        .map(|messages| ResponseJson(ApiResponse::success(messages)))
        .map_err(mail_error_response)
}

async fn list_awaiting_reply(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<WorkspaceQuery>,
) -> MailRouteResult<Vec<AwaitingReplyItem>> {
    list_workspace_awaiting_reply(&deployment.db().pool, query.workspace_id)
        .await
        .map(|messages| ResponseJson(ApiResponse::success(messages)))
        .map_err(mail_error_response)
}

#[derive(Debug, Deserialize)]
struct AttachBlobRequest {
    inline_blob_path: String,
    mime_type: Option<String>,
    size_bytes: Option<i64>,
    filename: Option<String>,
}

async fn attach_blob(
    State(deployment): State<DeploymentImpl>,
    Path(message_id): Path<Uuid>,
    ResponseJson(payload): ResponseJson<AttachBlobRequest>,
) -> MailRouteResult<MailAttachment> {
    let req = CreateMailAttachment {
        message_id,
        inline_blob_path: payload.inline_blob_path,
        mime_type: payload.mime_type,
        size_bytes: payload.size_bytes,
        filename: payload.filename,
    };
    attach_blob_to_message(&deployment.db().pool, req)
        .await
        .map(|att| ResponseJson(ApiResponse::success(att)))
        .map_err(mail_error_response)
}

async fn list_attachments(
    State(deployment): State<DeploymentImpl>,
    Path(message_id): Path<Uuid>,
) -> MailRouteResult<Vec<MailAttachment>> {
    list_attachments_for_message(&deployment.db().pool, message_id)
        .await
        .map(|atts| ResponseJson(ApiResponse::success(atts)))
        .map_err(mail_error_response)
}

fn mail_error_response(error: MailError) -> (StatusCode, ResponseJson<serde_json::Value>) {
    match error {
        MailError::AlreadyResponded => (
            StatusCode::CONFLICT,
            ResponseJson(json!({ "error": "already_responded" })),
        ),
        MailError::NotFound => (
            StatusCode::NOT_FOUND,
            ResponseJson(json!({ "error": "not_found" })),
        ),
        MailError::InvalidRequest(message) => (
            StatusCode::BAD_REQUEST,
            ResponseJson(json!({ "error": "invalid_request", "message": message })),
        ),
        MailError::Database(error) => {
            tracing::error!(?error, "mail route database error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(json!({ "error": "internal_server_error" })),
            )
        }
    }
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/mail/send", post(send))
        .route("/mail/broadcast", post(broadcast))
        .route("/mail/threads", get(list_threads))
        .route("/mail/threads/{thread_id}", get(get_thread))
        .route("/mail/messages/{message_id}", get(get_message))
        .route("/mail/messages/{message_id}/reply", post(reply))
        .route("/mail/inbox/unread", get(list_unread))
        .route("/mail/awaiting-reply", get(list_awaiting_reply))
        .route(
            "/mail/messages/{message_id}/attachments",
            get(list_attachments).post(attach_blob),
        )
}
