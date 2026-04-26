import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type {
  ApiResponse,
  AutomationRule,
  CreateAutomationRule,
  FireAutomationResult,
  SafetyConfig,
  UpdateAutomationRule,
  UpdateSafetyConfig,
} from 'shared/types';
import { makeLocalApiRequest } from '@/shared/lib/localApiTransport';

async function call<T>(
  path: string,
  init?: RequestInit & { json?: unknown }
): Promise<T> {
  const { json, ...rest } = init ?? {};
  const headers = new Headers(rest.headers ?? {});
  if (json !== undefined && !headers.has('content-type')) {
    headers.set('content-type', 'application/json');
  }
  const res = await makeLocalApiRequest(path, {
    ...rest,
    headers,
    body: json !== undefined ? JSON.stringify(json) : rest.body,
  });
  if (!res.ok) {
    throw new Error(`${res.status} ${res.statusText}`);
  }
  const payload = (await res.json()) as ApiResponse<T>;
  if (!payload.success || payload.data == null) {
    throw new Error(payload.message ?? 'request failed');
  }
  return payload.data;
}

const automationsKey = (workspaceId?: string) =>
  workspaceId ? ['automations', workspaceId] : ['automations'];

export function useAutomations(workspaceId?: string) {
  return useQuery({
    queryKey: automationsKey(workspaceId),
    queryFn: () => {
      const qs = workspaceId
        ? `?workspace_id=${encodeURIComponent(workspaceId)}`
        : '';
      return call<AutomationRule[]>(`/api/automations${qs}`);
    },
  });
}

export function useCreateAutomation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateAutomationRule) =>
      call<AutomationRule>('/api/automations', {
        method: 'POST',
        json: input,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['automations'] });
    },
  });
}

export function useUpdateAutomation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, patch }: { id: string; patch: UpdateAutomationRule }) =>
      call<AutomationRule>(`/api/automations/${id}`, {
        method: 'PATCH',
        json: patch,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['automations'] });
    },
  });
}

export function useDeleteAutomation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      const res = await makeLocalApiRequest(`/api/automations/${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) {
        throw new Error(`${res.status} ${res.statusText}`);
      }
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['automations'] });
    },
  });
}

export function useFireAutomation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      call<FireAutomationResult>(`/api/automations/${id}/fire`, {
        method: 'POST',
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['automations'] });
      qc.invalidateQueries({ queryKey: ['dispatches'] });
    },
  });
}

export function useSafetyConfig() {
  return useQuery({
    queryKey: ['safety', 'config'],
    queryFn: () => call<SafetyConfig>('/api/safety/config'),
  });
}

export function useUpdateSafetyConfig() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patch: UpdateSafetyConfig) =>
      call<SafetyConfig>('/api/safety/config', {
        method: 'PATCH',
        json: patch,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['safety'] });
    },
  });
}
