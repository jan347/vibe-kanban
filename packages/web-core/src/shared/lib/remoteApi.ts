// TODO(local-first): remoteApi is dead. Stubbed surface for legacy callers.

export function getRemoteApiUrl(): string {
  return '';
}

export async function makeRequest(
  _path: string,
  _init?: RequestInit
): Promise<Response> {
  return new Response(null, { status: 503 });
}

export async function fetchAttachmentSasUrl(
  _attachmentId: string
): Promise<string> {
  throw new Error('Remote attachments are unavailable in local-first mode');
}

export interface BulkUpdateIssueItem {
  id: string;
  changes: Record<string, unknown>;
}

export async function bulkUpdateIssues(_payload: unknown): Promise<void> {
  throw new Error('Bulk issue update is unavailable in local-first mode');
}

// TODO(local-first): attachment helpers stubbed; remote storage is gone.
export async function commitCommentAttachments(
  _commentId: string,
  _attachmentIds: string[]
): Promise<void> {
  return;
}

export async function commitIssueAttachments(
  _issueId: string,
  _attachmentIds: string[]
): Promise<void> {
  return;
}

export async function deleteAttachment(_attachmentId: string): Promise<void> {
  return;
}

export interface AttachmentInitResponse {
  upload_url: string;
  attachment_id: string;
  blob_id: string;
}

export async function initAttachmentUpload(
  _params: Record<string, unknown>
): Promise<AttachmentInitResponse> {
  throw new Error(
    'Remote attachment upload is unavailable in local-first mode'
  );
}

export async function confirmAttachmentUpload(
  _attachmentId: string
): Promise<void> {
  return;
}

export async function uploadToAzure(
  _url: string,
  _file: Blob,
  _onProgress?: (pct: number) => void
): Promise<void> {
  throw new Error(
    'Remote attachment upload is unavailable in local-first mode'
  );
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
