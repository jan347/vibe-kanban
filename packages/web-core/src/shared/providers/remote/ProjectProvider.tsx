// TODO(local-first): ProjectProvider remote-context is dead. Stub renders children.
import type { ReactNode } from 'react';

export function ProjectProvider({
  children,
}: {
  projectId?: string;
  children: ReactNode;
}) {
  return <>{children}</>;
}
