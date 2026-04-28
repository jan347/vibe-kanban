// TODO(local-first): remote cloud hosts are dead. Single-user local mode.

export interface RemoteCloudHost {
  id: string;
  name: string;
  nickname: string | null;
  host_id: string;
  status: 'online' | 'offline';
}

export interface RemoteCloudHostsState {
  hosts: RemoteCloudHost[];
}

export function useRemoteCloudHostsAppBarModel(): {
  hosts: RemoteCloudHost[];
} {
  return { hosts: [] };
}

export function useRemoteCloudHostsState(): {
  data: RemoteCloudHostsState;
  isLoading: boolean;
} {
  return { data: { hosts: [] }, isLoading: false };
}
