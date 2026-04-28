import { useEffect } from 'react';
import { useUserSystem } from '@/shared/hooks/useUserSystem';
import { useAppNavigation } from '@/shared/hooks/useAppNavigation';

// TODO(local-first): redirect collapses to "go to workspaces"; orgs/onboarding
// flows have been removed in single-user local mode.
export function RootRedirectPage() {
  const { config, loading } = useUserSystem();
  const appNavigation = useAppNavigation();

  useEffect(() => {
    if (loading || !config) {
      return;
    }
    appNavigation.goToWorkspaces({ replace: true });
  }, [appNavigation, config, loading]);

  return (
    <div className="h-screen bg-primary flex items-center justify-center">
      <p className="text-low">Loading...</p>
    </div>
  );
}
