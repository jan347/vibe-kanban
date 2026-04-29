export type AppDestination =
  | { kind: 'root' }
  | { kind: 'workspaces'; hostId?: string }
  | { kind: 'workspaces-create'; hostId?: string }
  | { kind: 'workspace'; workspaceId: string; hostId?: string }
  | { kind: 'workspace-vscode'; workspaceId: string; hostId?: string }
  | { kind: 'automation' }
  | { kind: 'mail' }
  | { kind: 'friction' };

export type NavigationTransition = {
  replace?: boolean;
};

export interface AppNavigation {
  resolveFromPath(path: string): AppDestination | null;
  goToRoot(transition?: NavigationTransition): void;
  goToWorkspaces(transition?: NavigationTransition): void;
  goToWorkspacesCreate(transition?: NavigationTransition): void;
  goToWorkspace(workspaceId: string, transition?: NavigationTransition): void;
  goToWorkspaceVsCode(
    workspaceId: string,
    transition?: NavigationTransition
  ): void;
  goToAutomation(transition?: NavigationTransition): void;
  goToMail(transition?: NavigationTransition): void;
  goToFriction(transition?: NavigationTransition): void;
}

type WorkspaceDestinationKind =
  | 'workspaces'
  | 'workspaces-create'
  | 'workspace'
  | 'workspace-vscode';

export type WorkspaceDestination = Extract<
  AppDestination,
  { kind: WorkspaceDestinationKind }
>;

export function getDestinationHostId(
  destination: AppDestination | null
): string | null {
  if (!destination || !('hostId' in destination)) {
    return null;
  }

  return destination.hostId ?? null;
}

export function isWorkspacesDestination(
  destination: AppDestination | null
): destination is WorkspaceDestination {
  if (!destination) {
    return false;
  }

  switch (destination.kind) {
    case 'workspaces':
    case 'workspaces-create':
    case 'workspace':
    case 'workspace-vscode':
      return true;
    default:
      return false;
  }
}

export function isLocalWorkspacesDestination(
  destination: AppDestination | null
): destination is WorkspaceDestination {
  return (
    isWorkspacesDestination(destination) &&
    getDestinationHostId(destination) === null
  );
}

export function isRemoteWorkspacesDestination(
  destination: AppDestination | null
): destination is WorkspaceDestination {
  return (
    isWorkspacesDestination(destination) &&
    getDestinationHostId(destination) !== null
  );
}
