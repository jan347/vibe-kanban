// TODO(local-first): organizations are dead in single-user local mode.
// This stub keeps the surface area compiling for consumers we have not yet
// rewritten; it always returns an empty list.

export interface UserOrganizationsResult {
  organizations: Array<{ id: string; name: string; is_personal: boolean }>;
}

export function useUserOrganizations(): {
  data: UserOrganizationsResult | undefined;
  isLoading: boolean;
} {
  return {
    data: { organizations: [] },
    isLoading: false,
  };
}
