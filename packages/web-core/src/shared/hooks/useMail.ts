import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { MailMessage, MailRecipient, MailThread } from 'shared/types';
import { makeLocalApiRequest } from '@/shared/lib/localApiTransport';
import type { ApiResponse } from 'shared/types';

// Composite shapes returned by the new mail routes.

export interface MailMessageWithRecipients {
  message: MailMessage;
  recipients: MailRecipient[];
}

export interface MailThreadWithMessages {
  thread: MailThread;
  messages: MailMessageWithRecipients[];
}

export interface UnreadMailItem {
  workspace_id: string | null;
  recipient_id: string;
  recipient_kind: 'workspace' | 'human';
  message_id: string;
  thread_id: string;
  subject: string;
  sender: {
    kind: 'workspace' | 'human';
    workspace_id: string | null;
    execution_process_id: string | null;
  };
  requires_response: boolean;
  created_at: string;
}

export interface AwaitingReplyItem {
  workspace_id: string;
  message_id: string;
  thread_id: string;
  subject: string;
  requires_response: boolean;
  created_at: string;
}

export interface MailThreadSummary {
  thread_id: string;
  subject: string;
  kind: MailThread['kind'];
  last_message_at: string;
  unread_count: number;
  created_at: string;
}

async function unwrap<T>(res: Response): Promise<T> {
  if (!res.ok) {
    throw new Error(`Mail API error ${res.status}: ${res.statusText}`);
  }
  const body = (await res.json()) as ApiResponse<T>;
  if (!body.success || body.data === undefined || body.data === null) {
    throw new Error(body.message ?? 'Mail API returned unsuccessful response');
  }
  return body.data;
}

export const mailKeys = {
  all: ['mail'] as const,
  unread: (workspaceId?: string) =>
    ['mail', 'unread', workspaceId ?? 'human'] as const,
  awaiting: (workspaceId?: string) =>
    ['mail', 'awaiting', workspaceId ?? 'all'] as const,
  threads: (workspaceId?: string) =>
    ['mail', 'threads', workspaceId ?? 'all'] as const,
  thread: (threadId: string) => ['mail', 'thread', threadId] as const,
  message: (messageId: string) => ['mail', 'message', messageId] as const,
};

export function useUnreadMail(workspaceId?: string) {
  return useQuery({
    queryKey: mailKeys.unread(workspaceId),
    queryFn: async () => {
      const qs = workspaceId ? `?workspace_id=${workspaceId}` : '';
      const res = await makeLocalApiRequest(`/api/mail/inbox/unread${qs}`);
      return unwrap<UnreadMailItem[]>(res);
    },
    refetchInterval: 5000,
    refetchOnWindowFocus: true,
  });
}

export function useAwaitingReply(workspaceId?: string) {
  return useQuery({
    queryKey: mailKeys.awaiting(workspaceId),
    queryFn: async () => {
      const qs = workspaceId ? `?workspace_id=${workspaceId}` : '';
      const res = await makeLocalApiRequest(`/api/mail/awaiting-reply${qs}`);
      return unwrap<AwaitingReplyItem[]>(res);
    },
    refetchInterval: 5000,
    enabled: !!workspaceId, // awaiting-reply requires workspace scope
    refetchOnWindowFocus: true,
  });
}

export function useMailThreads(workspaceId?: string) {
  return useQuery({
    queryKey: mailKeys.threads(workspaceId),
    queryFn: async () => {
      const qs = workspaceId ? `?workspace_id=${workspaceId}` : '';
      const res = await makeLocalApiRequest(`/api/mail/threads${qs}`);
      return unwrap<MailThreadSummary[]>(res);
    },
    refetchInterval: 10000,
  });
}

export function useMailThread(threadId: string | undefined) {
  return useQuery({
    queryKey: mailKeys.thread(threadId ?? ''),
    queryFn: async () => {
      const res = await makeLocalApiRequest(`/api/mail/threads/${threadId}`);
      return unwrap<MailThreadWithMessages>(res);
    },
    enabled: !!threadId,
    refetchInterval: 5000,
  });
}

export interface ReplyArgs {
  message_id: string;
  recipient_id: string;
  response_value: unknown; // JSON-serializable; will be JSON.stringify'd
}

export function useReplyToMail() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      message_id,
      recipient_id,
      response_value,
    }: ReplyArgs) => {
      const res = await makeLocalApiRequest(
        `/api/mail/messages/${message_id}/reply`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            recipient_id,
            response_value_json: JSON.stringify(response_value),
          }),
        }
      );
      if (res.status === 409) {
        throw new Error('already_responded');
      }
      return unwrap<{ ok: boolean }>(res);
    },
    onSuccess: () => {
      // Invalidate everything mail-related so the inbox + thread + awaiting-reply views refresh
      queryClient.invalidateQueries({ queryKey: mailKeys.all });
    },
  });
}
