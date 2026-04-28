// TODO(local-first): OrgContext is dead in single-user local mode. Stubbed
// for legacy consumers (e.g. KanbanContainer).

import type { Project } from 'shared/remote-types';

export interface OrgMemberWithProfile {
  user_id: string;
  email?: string | null;
  first_name?: string | null;
  last_name?: string | null;
  username?: string | null;
  avatar_url?: string | null;
  role?: 'member' | 'admin' | 'owner' | null;
}

export interface OrgContextValue {
  membersWithProfilesById: Map<string, OrgMemberWithProfile>;
  projects: Project[];
  isLoading: boolean;
}

export function useOrgContext(): OrgContextValue {
  return {
    membersWithProfilesById: new Map(),
    projects: [],
    isLoading: false,
  };
}
