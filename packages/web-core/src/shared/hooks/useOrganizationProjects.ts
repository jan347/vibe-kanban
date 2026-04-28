// TODO(local-first): organization-scoped projects are dead in single-user
// local mode. Stub returns an empty list so legacy consumers still compile.

import type { Project } from 'shared/remote-types';

export type OrgProject = Project;

export function useOrganizationProjects(_orgId: string | null | undefined): {
  data: OrgProject[];
  isLoading: boolean;
} {
  return { data: [], isLoading: false };
}
