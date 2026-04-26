import { useMemo, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { ArrowLeftIcon, PaperPlaneIcon } from '@phosphor-icons/react';
import {
  useMailThread,
  useReplyToMail,
  type MailMessageWithRecipients,
} from '@/shared/hooks/useMail';
import type { MailRecipient, MailResponseKind } from 'shared/types';

function formatTimestamp(iso: string): string {
  return new Date(iso).toLocaleString();
}

interface ResponseOption {
  key: string;
  label: string;
  description?: string;
}

function parseResponseOptions(json: string | null): ResponseOption[] {
  if (!json) return [];
  try {
    const parsed = JSON.parse(json);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (o): o is ResponseOption =>
        typeof o === 'object' &&
        o !== null &&
        typeof (o as ResponseOption).key === 'string' &&
        typeof (o as ResponseOption).label === 'string'
    );
  } catch {
    return [];
  }
}

function ReplyForm({
  recipient,
  messageId,
  responseKind,
  responseOptionsJson,
  onSent,
}: {
  recipient: MailRecipient;
  messageId: string;
  responseKind: MailResponseKind | null;
  responseOptionsJson: string | null;
  onSent: () => void;
}) {
  const [text, setText] = useState('');
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const reply = useReplyToMail();

  const options = useMemo(
    () => parseResponseOptions(responseOptionsJson),
    [responseOptionsJson]
  );

  if (recipient.responded_at) {
    if (recipient.response_value_json === null) {
      return (
        <div className="rounded-sm border border-secondary bg-panel p-3 text-low">
          Expired without reply.
        </div>
      );
    }
    return (
      <div className="rounded-sm border border-secondary bg-panel p-3 text-low">
        Already replied at {formatTimestamp(recipient.responded_at)}.
      </div>
    );
  }

  const submit = async () => {
    setError(null);
    let value: unknown = null;
    if (responseKind === 'options') {
      if (!selectedKey) {
        setError('Pick an option first.');
        return;
      }
      value = { key: selectedKey };
    } else if (responseKind === 'approval') {
      if (selectedKey !== 'approve' && selectedKey !== 'reject') {
        setError('Approve or reject first.');
        return;
      }
      value = { decision: selectedKey };
    } else if (responseKind === 'free_text' || responseKind === null || responseKind === 'none') {
      if (!text.trim()) {
        setError('Reply cannot be empty.');
        return;
      }
      value = { text: text.trim() };
    } else if (responseKind === 'file') {
      // File uploads not yet supported in Phase 4a; degrade to text.
      if (!text.trim()) {
        setError('File uploads coming in Phase 4c. For now reply with text.');
        return;
      }
      value = { text_in_lieu_of_file: text.trim() };
    }

    try {
      await reply.mutateAsync({
        message_id: messageId,
        recipient_id: recipient.id,
        response_value: value,
      });
      setText('');
      setSelectedKey(null);
      onSent();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Reply failed.');
    }
  };

  return (
    <div className="flex flex-col gap-2 rounded-sm border border-secondary bg-panel p-3">
      <span className="text-low">Reply</span>
      {responseKind === 'options' && options.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {options.map((opt) => (
            <button
              key={opt.key}
              onClick={() => setSelectedKey(opt.key)}
              className={`rounded-sm border px-3 py-1 text-base ${
                selectedKey === opt.key
                  ? 'border-brand bg-brand text-high'
                  : 'border-secondary bg-panel text-high hover:bg-secondary'
              }`}
              title={opt.description}
            >
              {opt.label}
            </button>
          ))}
        </div>
      )}
      {responseKind === 'approval' && (
        <div className="flex gap-2">
          <button
            onClick={() => setSelectedKey('approve')}
            className={`rounded-sm border px-3 py-1 text-base ${
              selectedKey === 'approve'
                ? 'border-success bg-success text-high'
                : 'border-secondary bg-panel text-high hover:bg-secondary'
            }`}
          >
            Approve
          </button>
          <button
            onClick={() => setSelectedKey('reject')}
            className={`rounded-sm border px-3 py-1 text-base ${
              selectedKey === 'reject'
                ? 'border-error bg-error text-high'
                : 'border-secondary bg-panel text-high hover:bg-secondary'
            }`}
          >
            Reject
          </button>
        </div>
      )}
      {(responseKind === null ||
        responseKind === 'none' ||
        responseKind === 'free_text' ||
        responseKind === 'file') && (
        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          rows={3}
          placeholder="Type your reply..."
          className="w-full resize-none rounded-sm border border-secondary bg-background p-2 text-base text-high"
        />
      )}
      {error && <span className="text-error">{error}</span>}
      <div className="flex justify-end">
        <button
          onClick={submit}
          disabled={reply.isPending}
          className="flex items-center gap-1 rounded-sm bg-brand px-3 py-1 text-base text-high hover:bg-brand-hover disabled:opacity-50"
        >
          <PaperPlaneIcon className="size-4" />
          {reply.isPending ? 'Sending…' : 'Send reply'}
        </button>
      </div>
    </div>
  );
}

function MessageCard({
  item,
  onReplied,
}: {
  item: MailMessageWithRecipients;
  onReplied: () => void;
}) {
  const { message, recipients } = item;
  // For Phase 4a single-recipient: pick the first non-responded recipient as the reply target.
  const pendingRecipient = recipients.find((r) => !r.responded_at);

  return (
    <article className="flex flex-col gap-2 rounded-sm border border-secondary bg-panel p-4">
      <header className="flex items-baseline justify-between">
        <span className="text-base text-high">
          {message.sender_kind === 'workspace' ? 'Agent' : 'Human'}
          {message.sender_kind === 'workspace' && message.sender_workspace_id
            ? ` (${message.sender_workspace_id.slice(0, 8)}…)`
            : ''}
        </span>
        <span className="text-low">{formatTimestamp(message.created_at)}</span>
      </header>
      <div className="whitespace-pre-wrap text-base text-high">
        {message.body}
      </div>
      {message.requires_response && pendingRecipient && (
        <ReplyForm
          recipient={pendingRecipient}
          messageId={message.id}
          responseKind={message.response_kind ?? null}
          responseOptionsJson={message.response_options_json ?? null}
          onSent={onReplied}
        />
      )}
    </article>
  );
}

interface MailThreadViewProps {
  threadId: string;
}

export function MailThreadView({ threadId }: MailThreadViewProps) {
  const navigate = useNavigate();
  const { data, isLoading, error, refetch } = useMailThread(threadId);

  if (isLoading) {
    return (
      <div className="flex h-full flex-1 items-center justify-center bg-primary">
        <span className="text-low">Loading thread…</span>
      </div>
    );
  }
  if (error || !data) {
    return (
      <div className="flex h-full flex-1 items-center justify-center bg-primary">
        <span className="text-low">
          Thread unavailable
          {error ? `: ${error instanceof Error ? error.message : ''}` : '.'}
        </span>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-1 flex-col gap-4 overflow-y-auto bg-primary p-6">
      <header className="flex items-center gap-2">
        <button
          onClick={() => navigate({ to: '/mail' as never })}
          className="flex items-center gap-1 rounded-sm px-2 py-1 text-base text-low hover:bg-secondary hover:text-high"
        >
          <ArrowLeftIcon className="size-4" />
          Inbox
        </button>
        <h1 className="text-lg text-high">{data.thread.subject}</h1>
      </header>
      <ul className="flex flex-col gap-3">
        {data.messages.map((m) => (
          <li key={m.message.id}>
            <MessageCard item={m} onReplied={() => refetch()} />
          </li>
        ))}
      </ul>
    </div>
  );
}
