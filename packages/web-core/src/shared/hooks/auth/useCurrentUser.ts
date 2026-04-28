// TODO(local-first): current user lookup is dead. Local single-user mode.

export interface LocalUser {
  user_id: string;
  email: string | null;
  first_name: string | null;
  last_name: string | null;
  username: string | null;
}

export function useCurrentUser(): {
  data: LocalUser | undefined;
  isLoading: boolean;
} {
  return {
    data: {
      user_id: 'local',
      email: null,
      first_name: null,
      last_name: null,
      username: 'local',
    },
    isLoading: false,
  };
}
