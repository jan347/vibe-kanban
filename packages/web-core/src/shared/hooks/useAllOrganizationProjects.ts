// TODO(local-first): organization-scoped projects are dead in single-user
// local mode. Stub returns an empty list so legacy consumers still compile.

import type { OrgProject } from './useOrganizationProjects';

export function useAllOrganizationProjects(_options?: { enabled?: boolean }): {
  data: OrgProject[];
  isLoading: boolean;
} {
  return { data: [], isLoading: false };
}
