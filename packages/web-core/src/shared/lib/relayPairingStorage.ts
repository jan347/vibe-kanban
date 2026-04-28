// TODO(local-first): relay pairings are dead in single-user mode.

export interface PairedRelayHost {
  id: string;
  host_id: string;
  nickname: string | null;
}

export function listPairedRelayHosts(): PairedRelayHost[] {
  return [];
}
