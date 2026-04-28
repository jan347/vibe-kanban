// TODO(local-first): create-remote-project dialog is dead in single-user mode.

export interface CreateRemoteProjectResult {
  action: 'cancelled' | 'created';
  project?: { id: string; name: string };
}

export const CreateRemoteProjectDialog = {
  show: async (_props: {
    organizationId: string;
  }): Promise<CreateRemoteProjectResult> => {
    return { action: 'cancelled' };
  },
};
