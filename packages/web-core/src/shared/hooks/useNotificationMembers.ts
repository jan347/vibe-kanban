// TODO(local-first): notification members are dead in single-user local mode.

export interface NotificationMember {
  id: string;
  user_id: string;
  display_name: string;
  first_name?: string | null;
  last_name?: string | null;
  username?: string | null;
  avatar_url?: string | null;
}

export interface NotificationMembersResult {
  members: NotificationMember[];
  membersByUserId: Map<string, NotificationMember>;
}

export function useNotificationMembers(_options?: { enabled?: boolean }): {
  data: NotificationMembersResult;
  isLoading: boolean;
} {
  return {
    data: { members: [], membersByUserId: new Map() },
    isLoading: false,
  };
}
