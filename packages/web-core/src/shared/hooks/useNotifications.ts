// Local-first stub. The original hook synced notifications via Electric
// against the multi-tenant backend; that surface is gone. In single-user
// local mode there are no inter-user notifications, so we return an
// empty list and a no-op updater. AppBarNotificationBellContainer and
// NotificationsPage render their empty/zero states with this shape.
import type { GroupedNotification } from '@/shared/lib/notifications';

interface NotificationRow {
  id: string;
  seen: boolean;
}

interface NotificationUpdate {
  id: string;
  changes: Partial<NotificationRow>;
}

interface UseNotificationsResult {
  data: NotificationRow[];
  enabled: boolean;
  unseenCount: number;
  groupedNotifications: GroupedNotification[];
  updateMany: (_updates: NotificationUpdate[]) => void;
  isLoading: boolean;
  error: null;
}

export function useNotifications(): UseNotificationsResult {
  return {
    data: [],
    enabled: false,
    unseenCount: 0,
    groupedNotifications: [],
    updateMany: () => {},
    isLoading: false,
    error: null,
  };
}
