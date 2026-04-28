import { useMemo, type ReactNode } from 'react';
import {
  AuthContext,
  type AuthContextValue,
} from '@/shared/hooks/auth/useAuth';

interface LocalAuthProviderProps {
  children: ReactNode;
}

// Local-first single-user mode: always signed in as the local user.
export function LocalAuthProvider({ children }: LocalAuthProviderProps) {
  const value = useMemo<AuthContextValue>(
    () => ({
      isSignedIn: true,
      isLoaded: true,
      userId: 'local',
    }),
    []
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
