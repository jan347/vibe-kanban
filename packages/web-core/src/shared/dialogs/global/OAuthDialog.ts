// TODO(local-first): OAuth dialog is dead in single-user local mode.

export const OAuthDialog = {
  show: async (_props: Record<string, unknown>): Promise<void> => {
    // no-op in local-first mode
  },
};
