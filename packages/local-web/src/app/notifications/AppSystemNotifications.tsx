import { useEffect, useRef } from 'react';
import { useNotifications } from '@/shared/hooks/useNotifications';
import { getGroupedNotificationText } from '@/shared/lib/notificationMessage';
import { showSystemNotification } from '@web/app/notifications/showSystemNotification';

export function AppSystemNotifications() {
  const { enabled, groupedNotifications } = useNotifications();
  const displayedNotificationIdsRef = useRef(new Set<string>());
  const initializedRef = useRef(false);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    if (!initializedRef.current) {
      for (const group of groupedNotifications) {
        if (!group.seen) {
          displayedNotificationIdsRef.current.add(group.id);
        }
      }
      initializedRef.current = true;
      return;
    }

    const activeGroupIds = new Set(
      groupedNotifications.map((group) => group.id)
    );
    for (const id of displayedNotificationIdsRef.current) {
      if (!activeGroupIds.has(id)) {
        displayedNotificationIdsRef.current.delete(id);
      }
    }

    for (const group of groupedNotifications) {
      if (group.seen || displayedNotificationIdsRef.current.has(group.id)) {
        continue;
      }

      displayedNotificationIdsRef.current.add(group.id);
      void showSystemNotification({
        id: group.id,
        title: 'GenCap Control Room',
        body: getGroupedNotificationText(group),
        deeplinkPath: group.deeplinkPath ?? undefined,
      });
    }
  }, [enabled, groupedNotifications]);

  return null;
}
