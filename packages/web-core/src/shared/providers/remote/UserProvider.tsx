// Local-first stub. The original UserProvider hydrated user-scoped data
// from Electric sync (remote-only workspaces). In single-user local mode
// the equivalents either come from the local backend (workspaces) or
// don't apply (multi-tenant sync errors). We supply an inert context
// value so consumers (NavbarContainer, ActionsProvider, CreateModeProvider)
// don't throw — they each fall back to local-only behavior on empty data.
import type { ReactNode } from 'react';
import { useMemo } from 'react';
import {
  UserContext,
  type UserContextValue,
} from '@/shared/hooks/useUserContext';

export function UserProvider({ children }: { children: ReactNode }) {
  const value = useMemo<UserContextValue>(
    () => ({
      workspaces: [],
      isLoading: false,
      error: null,
      retry: () => {},
      getWorkspacesForIssue: () => [],
    }),
    []
  );
  return <UserContext.Provider value={value}>{children}</UserContext.Provider>;
}
