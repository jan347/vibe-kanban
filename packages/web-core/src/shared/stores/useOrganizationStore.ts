// TODO(local-first): organization store is dead. Stub returns null/no-op
// so legacy consumers still compile.

interface OrganizationStoreState {
  selectedOrgId: string | null;
  setSelectedOrgId: (id: string | null) => void;
  clearSelectedOrgId: () => void;
}

const NO_OP_STATE: OrganizationStoreState = {
  selectedOrgId: null,
  setSelectedOrgId: () => {},
  clearSelectedOrgId: () => {},
};

interface UseOrganizationStore {
  <T>(selector: (state: OrganizationStoreState) => T): T;
  getState: () => OrganizationStoreState;
}

export const useOrganizationStore: UseOrganizationStore = Object.assign(
  <T>(selector: (state: OrganizationStoreState) => T): T =>
    selector(NO_OP_STATE),
  { getState: () => NO_OP_STATE }
);
