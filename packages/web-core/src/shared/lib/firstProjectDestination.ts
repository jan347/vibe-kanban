// TODO(local-first): first-project lookup is dead in single-user local mode.

export interface FirstProjectDestination {
  kind: 'project';
  projectId: string;
}

export async function getFirstProjectDestination(
  _setSelectedOrgId: (id: string | null) => void,
  _selectedOrgId?: string | null,
  _selectedProjectId?: string | null
): Promise<FirstProjectDestination | null> {
  return null;
}
