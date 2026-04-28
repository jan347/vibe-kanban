// Local-first stub surface. The original module spoke to the
// multi-tenant backend (attachments via Azure SAS, bulk issue mutates,
// relay host directory, Electric sync proxy). All of those are gone.
//
// We keep the function signatures so legacy callers compile, but every
// operation is a no-op or a tagged warning. Throwing here would crash
// any UI surface the strip missed; warning + safe default keeps the app
// alive and surfaces the call site in dev tools so we can prune it.

function warnDead(name: string): void {
  // eslint-disable-next-line no-console
  console.warn(
    `[local-first] remoteApi.${name}() called — no-op in single-user local mode.`
  );
}

export function getRemoteApiUrl(): string {
  return '';
}

export async function makeRequest(
  _path: string,
  _init?: RequestInit
): Promise<Response> {
  warnDead('makeRequest');
  return new Response(null, { status: 503 });
}

// Signature must accept (attachmentId, type) — passed directly into
// attachment-node's CreateAttachmentNodeOptions.fetchAttachmentUrl.
// Returns an empty string so the consumer's <img src=""> produces a
// broken-image icon rather than crashing on `null`.
export async function fetchAttachmentSasUrl(
  _attachmentId: string,
  _type?: 'file' | 'thumbnail'
): Promise<string> {
  warnDead('fetchAttachmentSasUrl');
  return '';
}

export interface BulkUpdateIssueItem {
  id: string;
  changes: Record<string, unknown>;
}

export async function bulkUpdateIssues(_payload: unknown): Promise<void> {
  warnDead('bulkUpdateIssues');
}

export async function commitCommentAttachments(
  _commentId: string,
  _attachmentIds: string[]
): Promise<void> {
  // No-op intentionally — comment attachments collapsed to local files.
}

export async function commitIssueAttachments(
  _issueId: string,
  _attachmentIds: string[]
): Promise<void> {
  // No-op intentionally.
}

export async function deleteAttachment(_attachmentId: string): Promise<void> {
  // No-op intentionally — attachment lifecycle is local-only.
}

export interface AttachmentInitResponse {
  upload_url: string;
  attachment_id: string;
  blob_id: string;
}

export async function initAttachmentUpload(
  _params: Record<string, unknown>
): Promise<AttachmentInitResponse> {
  warnDead('initAttachmentUpload');
  return { upload_url: '', attachment_id: '', blob_id: '' };
}

export async function confirmAttachmentUpload(
  _attachmentId: string
): Promise<void> {
  // No-op intentionally.
}

export async function uploadToAzure(
  _url: string,
  _file: Blob,
  _onProgress?: (pct: number) => void
): Promise<void> {
  warnDead('uploadToAzure');
}

export async function computeFileHash(_file: Blob): Promise<string> {
  return '';
}

export interface RelayHost {
  id: string;
  name: string;
  status: 'online' | 'offline';
}

export async function listRelayHosts(): Promise<RelayHost[]> {
  return [];
}
