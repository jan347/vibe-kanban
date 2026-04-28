// TODO(local-first): OrgProvider remote-context is dead. Stub renders children.
import type { ReactNode } from 'react';

export function OrgProvider({
  children,
}: {
  organizationId?: string;
  children: ReactNode;
}) {
  return <>{children}</>;
}
