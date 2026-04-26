import { useMemo } from 'react';
import { SpinnerIcon, PlusIcon } from '@phosphor-icons/react';
import { useAppNavigation } from '@/shared/hooks/useAppNavigation';
import {
  useWorkspaces,
  type SidebarWorkspace,
} from '@/shared/hooks/useWorkspaces';

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
      <div className="flex h-full flex-1 flex-col items-center justify-center gap-4 bg-primary p-8">
        <h2 className="text-lg text-high">No workspaces yet</h2>
        <p className="text-low">
          Create your first workspace to start an agent.
        </p>
        <button
          className="flex items-center gap-1 rounded-sm bg-brand px-4 py-2 text-base text-high hover:bg-brand-hover"
          onClick={() => appNavigation.goToWorkspacesCreate({})}
        >
          <PlusIcon className="size-4" />
          Create workspace
        </button>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-1 flex-col gap-6 overflow-y-auto bg-primary p-6">
      <header className="flex items-center justify-between">
        <h1 className="text-lg text-high">Workspaces</h1>
        <button
          className="flex items-center gap-1 rounded-sm bg-brand px-3 py-1 text-base text-high hover:bg-brand-hover"
          onClick={() => appNavigation.goToWorkspacesCreate({})}
        >
          <PlusIcon className="size-4" />
          New workspace
        </button>
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
