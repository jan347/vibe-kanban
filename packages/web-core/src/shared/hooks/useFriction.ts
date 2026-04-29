import { useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { ApiResponse, FrictionSnapshot } from 'shared/types';
import { makeLocalApiRequest } from '@/shared/lib/localApiTransport';

const FRICTION_SNAPSHOT_KEY = ['friction', 'snapshot'] as const;
const FRICTION_POLL_MS = 30_000;

async function fetchFrictionSnapshot(): Promise<FrictionSnapshot> {
  const res = await makeLocalApiRequest('/api/friction/snapshot', {
    method: 'GET',
  });
  if (!res.ok) {
    throw new Error(`${res.status} ${res.statusText}`);
  }
  const payload = (await res.json()) as ApiResponse<FrictionSnapshot>;
  if (!payload.success || payload.data == null) {
    throw new Error(payload.message ?? 'friction snapshot request failed');
  }
  return payload.data;
}

/**
 * Polls /api/friction/snapshot every 30s and refreshes on tab focus.
 *
 * Per D-AUTO-1: 30s polling cadence + visibilitychange refresh keep the
 * dashboard in sync without WebSocket plumbing. The header strip drives
 * "Last entry: N min ago" so refresh-on-focus matters more than fast polling.
 */
export function useFrictionSnapshot() {
  const qc = useQueryClient();

  useEffect(() => {
    function handleVisibility() {
      if (typeof document === 'undefined') return;
      if (document.visibilityState === 'visible') {
        void qc.invalidateQueries({ queryKey: FRICTION_SNAPSHOT_KEY });
      }
    }
    if (typeof document !== 'undefined') {
      document.addEventListener('visibilitychange', handleVisibility);
    }
    return () => {
      if (typeof document !== 'undefined') {
        document.removeEventListener('visibilitychange', handleVisibility);
      }
    };
  }, [qc]);

  return useQuery({
    queryKey: FRICTION_SNAPSHOT_KEY,
    queryFn: fetchFrictionSnapshot,
    refetchInterval: FRICTION_POLL_MS,
    refetchIntervalInBackground: false,
  });
}
