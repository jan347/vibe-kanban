import { usePageTitle } from '@/shared/hooks/usePageTitle';

interface ProjectSunsetPageProps {
  projectName?: string;
}

// Vestigial sunset page from vibe-kanban's multi-tenant shutdown era.
// Projects still work in local-first mode — this surface only renders
// when ProjectKanban hits a degenerate state (no remote project shape
// available). Local-first has no data-export endpoint and no shutdown
// blog post, so we drop both CTAs and just show a holding message.
export function ProjectSunsetPage({ projectName }: ProjectSunsetPageProps) {
  usePageTitle(projectName, 'Project unavailable');

  return (
    <div className="h-full w-full overflow-auto bg-primary">
      <div className="mx-auto flex min-h-full w-full max-w-3xl items-center px-base py-double">
        <div className="w-full rounded-sm border border-border bg-secondary p-double">
          <div className="space-y-base">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-low">
              Project
            </p>
            <div className="space-y-half">
              <h1 className="text-2xl font-semibold text-high">
                Project unavailable
              </h1>
              <p className="text-sm text-low">
                {projectName
                  ? `"${projectName}" is not available in local-first mode.`
                  : 'This project is not available in local-first mode.'}{' '}
                Use the workspace list in the sidebar to start a new agent
                run.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
