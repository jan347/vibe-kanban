import { useMemo } from 'react';
import {
  SpinnerIcon,
  PlusIcon,
  EnvelopeIcon,
  RobotIcon,
} from '@phosphor-icons/react';
import { useAppNavigation } from '@/shared/hooks/useAppNavigation';
import {
  useWorkspaces,
  type SidebarWorkspace,
} from '@/shared/hooks/useWorkspaces';
import { useUnreadMail } from '@/shared/hooks/useMail';

type DashboardGroup =
  | 'needsMe'
  | 'running'
  | 'errored'
  | 'recentlyCompleted'
  | 'idle';

const GROUP_LABELS: Record<DashboardGroup, string> = {
  needsMe: 'Needs me',
  running: 'Running',
  errored: 'Errored',
  recentlyCompleted: 'Recently completed',
  idle: 'Idle',
};

const GROUP_ORDER: DashboardGroup[] = [
  'needsMe',
  'running',
  'errored',
  'recentlyCompleted',
  'idle',
];

function groupOf(ws: SidebarWorkspace): DashboardGroup {
  const errored =
    ws.latestProcessStatus === 'failed' || ws.latestProcessStatus === 'killed';
  const needsMe =
    !!ws.hasPendingApproval ||
    errored ||
    (ws.prStatus === 'open' && !!ws.hasUnseenActivity) ||
    (!!ws.isRunning && ws.hasRunningDevServer === false);
  if (needsMe) return 'needsMe';
  if (ws.isRunning) return 'running';
  if (errored) return 'errored';
  if (ws.latestProcessStatus === 'completed') return 'recentlyCompleted';
  return 'idle';
}

function WorkspaceRow({
  ws,
  onSelect,
}: {
  ws: SidebarWorkspace;
  onSelect: () => void;
}) {
  return (
    <button
      onClick={onSelect}
      className="flex w-full items-center gap-3 rounded-sm border border-secondary bg-panel p-3 text-left hover:bg-secondary"
    >
      <div className="flex flex-1 flex-col gap-0 min-w-0">
        <span className="text-base text-high truncate">{ws.name}</span>
        <span className="text-low truncate">{ws.branch}</span>
      </div>
      <div className="flex items-center gap-1 shrink-0">
        {ws.hasPendingApproval && (
          <span className="rounded-full bg-error px-2 text-base text-high">
            approval
          </span>
        )}
        {(ws.latestProcessStatus === 'failed' ||
          ws.latestProcessStatus === 'killed') && (
          <span className="rounded-full bg-error px-2 text-base text-high">
            {ws.latestProcessStatus}
          </span>
        )}
        {ws.isRunning && (
          <span className="rounded-full bg-success px-2 text-base text-high">
            running
          </span>
        )}
        {ws.prStatus === 'open' && (
          <span className="rounded-full bg-brand px-2 text-base text-high">
            PR{ws.prNumber ? ` #${ws.prNumber}` : ''}
          </span>
        )}
      </div>
    </button>
  );
}

export function WorkspacesLanding() {
  const appNavigation = useAppNavigation();
  const { workspaces, isLoading } = useWorkspaces();
  const { data: unreadMail } = useUnreadMail();
  const unreadMailCount = unreadMail?.length ?? 0;

  const grouped = useMemo(() => {
    const buckets: Record<DashboardGroup, SidebarWorkspace[]> = {
      needsMe: [],
      running: [],
      errored: [],
      recentlyCompleted: [],
      idle: [],
    };
    for (const ws of workspaces) {
      if (ws.isArchived) continue;
      buckets[groupOf(ws)].push(ws);
    }
    return buckets;
  }, [workspaces]);

  if (isLoading) {
    return (
      <div className="flex h-full flex-1 items-center justify-center bg-primary">
        <SpinnerIcon className="size-6 animate-spin text-low" />
      </div>
    );
  }

  if (workspaces.length === 0) {
    return (
      <div className="flex h-full flex-1 items-center justify-center bg-primary p-8">
        <div className="flex w-full max-w-xl flex-col gap-6 rounded-sm border border-border bg-secondary p-8">
          <div className="space-y-1">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-low">
              GenCap Control Room
            </p>
            <h2 className="text-2xl font-semibold text-high">
              Spin up your first workspace
            </h2>
            <p className="text-sm text-low">
              A workspace is one task pointed at one agent. Pick a repo,
              describe the work, the supervisor decides what auto-runs and what
              escalates to you.
            </p>
          </div>
          <button
            className="inline-flex items-center justify-center gap-2 self-start rounded-sm bg-brand px-base py-half text-sm font-medium text-on-brand transition-colors hover:bg-brand-hover"
            onClick={() => appNavigation.goToWorkspacesCreate({})}
          >
            <PlusIcon className="size-icon-base" weight="bold" />
            Create workspace
          </button>
          <div className="border-t border-border pt-base">
            <p className="mb-half text-xs font-medium uppercase tracking-wide text-low">
              While you wait
            </p>
            <div className="flex flex-col gap-half text-sm">
              <button
                className="inline-flex items-center gap-2 self-start rounded-sm px-2 py-1 text-normal hover:bg-tertiary"
                onClick={() => appNavigation.goToAutomation()}
              >
                <RobotIcon className="size-icon-sm" weight="bold" />
                Configure Auto Mode policy
              </button>
              <button
                className="inline-flex items-center gap-2 self-start rounded-sm px-2 py-1 text-normal hover:bg-tertiary"
                onClick={() => appNavigation.goToMail()}
              >
                <EnvelopeIcon className="size-icon-sm" weight="bold" />
                Inbox
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-1 flex-col gap-6 overflow-y-auto bg-primary p-6">
      <header className="flex items-center justify-between">
        <h1 className="text-lg text-high">Workspaces</h1>
        <div className="flex items-center gap-2">
          <button
            className="flex items-center gap-1 rounded-sm border border-secondary bg-panel px-3 py-1 text-base text-high hover:bg-secondary"
            onClick={() => appNavigation.goToMail()}
          >
            <EnvelopeIcon className="size-4" />
            Inbox
            {unreadMailCount > 0 && (
              <span className="rounded-full bg-error px-2 text-base text-high">
                {unreadMailCount}
              </span>
            )}
          </button>
          <button
            className="flex items-center gap-1 rounded-sm bg-brand px-3 py-1 text-base text-high hover:bg-brand-hover"
            onClick={() => appNavigation.goToWorkspacesCreate({})}
          >
            <PlusIcon className="size-4" />
            New workspace
          </button>
        </div>
      </header>
      {GROUP_ORDER.map((group) => {
        const items = grouped[group];
        if (items.length === 0) return null;
        return (
          <section key={group} className="flex flex-col gap-2">
            <h2 className="text-base text-high">
              {GROUP_LABELS[group]}{' '}
              <span className="text-low">({items.length})</span>
            </h2>
            <ul className="flex flex-col gap-1">
              {items.map((ws) => (
                <li key={ws.id}>
                  <WorkspaceRow
                    ws={ws}
                    onSelect={() => appNavigation.goToWorkspace(ws.id)}
                  />
                </li>
              ))}
            </ul>
          </section>
        );
      })}
    </div>
  );
}
