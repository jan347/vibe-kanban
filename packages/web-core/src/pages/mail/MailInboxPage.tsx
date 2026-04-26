import { useMemo } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { EnvelopeIcon, EnvelopeOpenIcon } from '@phosphor-icons/react';
import {
  useUnreadMail,
  type UnreadMailItem,
} from '@/shared/hooks/useMail';

function formatRelative(iso: string): string {
  const d = new Date(iso);
  const ms = Date.now() - d.getTime();
  const sec = Math.floor(ms / 1000);
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return d.toLocaleDateString();
}

function MailRow({
  item,
  onSelect,
}: {
  item: UnreadMailItem;
  onSelect: () => void;
}) {
  return (
    <button
      onClick={onSelect}
      className="flex w-full items-start gap-3 rounded-sm border border-secondary bg-panel p-3 text-left hover:bg-secondary"
    >
      <EnvelopeIcon className="size-4 shrink-0 mt-1 text-low" />
      <div className="flex flex-1 flex-col gap-0 min-w-0">
        <div className="flex items-baseline gap-2">
          <span className="text-base text-high truncate">{item.subject}</span>
          <span className="text-low shrink-0">
            {formatRelative(item.created_at)}
          </span>
        </div>
        <div className="flex items-center gap-2 text-low">
          <span>
            from{' '}
            {item.sender.kind === 'workspace' ? 'agent' : 'human'}
          </span>
          {item.requires_response && (
            <span className="rounded-full bg-error px-2 text-base text-high">
              needs reply
            </span>
          )}
        </div>
      </div>
    </button>
  );
}

interface MailInboxPageProps {
  workspaceId?: string; // when scoped to a workspace; otherwise lists human-recipient mail
}

export function MailInboxPage({ workspaceId }: MailInboxPageProps) {
  const navigate = useNavigate();
  const { data: items, isLoading, error } = useUnreadMail(workspaceId);

  const sorted = useMemo(() => {
    return (items ?? [])
      .slice()
      .sort((a, b) =>
        a.created_at < b.created_at ? 1 : a.created_at > b.created_at ? -1 : 0
      );
  }, [items]);

  if (isLoading) {
    return (
      <div className="flex h-full flex-1 items-center justify-center bg-primary">
        <span className="text-low">Loading mail…</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-full flex-1 items-center justify-center bg-primary">
        <span className="text-low">
          Mail unavailable: {error instanceof Error ? error.message : 'unknown error'}
        </span>
      </div>
    );
  }

  if (sorted.length === 0) {
    return (
      <div className="flex h-full flex-1 flex-col items-center justify-center gap-2 bg-primary p-8">
        <EnvelopeOpenIcon className="size-8 text-low" />
        <h2 className="text-lg text-high">Inbox empty</h2>
        <p className="text-low">
          No unread mail{workspaceId ? ' for this workspace' : ''}. Agents will
          send messages here when they need a decision.
        </p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-1 flex-col gap-4 overflow-y-auto bg-primary p-6">
      <header className="flex items-center justify-between">
        <h1 className="text-lg text-high">
          Inbox{' '}
          <span className="text-low">
            ({sorted.length} unread)
          </span>
        </h1>
      </header>
      <ul className="flex flex-col gap-1">
        {sorted.map((item) => (
          <li key={item.recipient_id}>
            <MailRow
              item={item}
              onSelect={() =>
                navigate({
                  to: '/mail/threads/$threadId' as never,
                  params: { threadId: item.thread_id } as never,
                })
              }
            />
          </li>
        ))}
      </ul>
    </div>
  );
}
