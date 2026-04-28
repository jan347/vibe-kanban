// TODO(local-first): auth runtime is dead. Stubbed for legacy callers.

export interface AuthShapeHandle {
  pause: () => void;
  resume: () => void;
}

export interface AuthRuntime {
  getToken: () => Promise<string | null>;
  triggerRefresh: () => Promise<void>;
  registerShape: (handle: AuthShapeHandle) => void;
}

const NOOP_AUTH_RUNTIME: AuthRuntime = {
  getToken: async () => null,
  triggerRefresh: async () => {},
  registerShape: () => {},
};

export function getAuthRuntime(): AuthRuntime {
  return NOOP_AUTH_RUNTIME;
}
