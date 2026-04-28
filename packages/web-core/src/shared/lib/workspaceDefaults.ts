import { workspacesApi } from '@/shared/lib/api';

export interface WorkspaceDefaults {
  preferredRepos: Array<{ repo_id: string; target_branch: string | null }>;
}

interface RecentWorkspaceCandidate {
  id: string;
  updatedAt: string;
}

/**
 * Fetches workspace creation defaults from the most recent workspace.
 *
 * Local-first single-user mode does not have org/project scoping,
 * so we just look at the globally most-recent workspace.
 */
export async function getWorkspaceDefaults(
  recentWorkspaces: RecentWorkspaceCandidate[]
): Promise<WorkspaceDefaults | null> {
  const mostRecent = [...recentWorkspaces].sort(
    (a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
  )[0];

  if (!mostRecent) {
    return null;
  }

  try {
    const repos = await workspacesApi.getRepos(mostRecent.id);
    return {
      preferredRepos: repos.map((r) => ({
        repo_id: r.id,
        target_branch: r.target_branch,
      })),
    };
  } catch (err) {
    console.warn('Failed to fetch workspace defaults:', err);
    return null;
  }
}
