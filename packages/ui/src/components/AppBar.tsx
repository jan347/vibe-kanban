import type { ReactNode } from 'react';
import {
  LayoutIcon,
  EnvelopeIcon,
  RobotIcon,
  WarningIcon,
  type Icon,
} from '@phosphor-icons/react';
import { cn } from '../lib/cn';
import { Tooltip } from './Tooltip';

interface AppBarProps {
  onWorkspacesClick: () => void;
  showWorkspacesButton?: boolean;
  isWorkspacesActive: boolean;
  onHoverStart?: () => void;
  onHoverEnd?: () => void;
  notificationBell?: ReactNode;
  appVersion?: string | null;
  updateVersion?: string | null;
  onUpdateClick?: () => void;
  // Local-first orchestration nav.
  onAutomationClick?: () => void;
  isAutomationActive?: boolean;
  onMailClick?: () => void;
  isMailActive?: boolean;
  onFrictionClick?: () => void;
  isFrictionActive?: boolean;
}

// Retained as a re-exported type so WorkspacesSidebar / SharedAppLayout
// keep compiling. The Remote section in the sidebar is gone in
// local-first; if we ever bring host pairing back this is the type to
// share again.
export type AppBarHostStatus = 'online' | 'offline' | 'unpaired';

function AppBarSectionLabel({ children }: { children: ReactNode }) {
  return (
    <p className="w-10 text-center text-[9px] font-medium leading-none tracking-wide text-low">
      {children}
    </p>
  );
}

const appBarItemBaseClassName =
  'flex items-center justify-center w-10 h-10 rounded-lg text-sm font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-brand';

type AppBarSection = {
  key: 'local';
  label: string;
  items: AppBarSectionItem[];
};

type AppBarSectionItem = {
  key: string;
  kind: 'icon-button';
  label: string;
  icon: Icon;
  isActive?: boolean;
  onClick?: () => void;
  className?: string;
  wrapperClassName?: string;
};

function getStandardAppBarButtonClassName({
  isActive = false,
  className,
}: {
  isActive?: boolean;
  className?: string;
}) {
  return cn(
    appBarItemBaseClassName,
    'cursor-pointer',
    isActive
      ? 'bg-brand/20 text-brand hover:bg-brand/20'
      : 'bg-primary text-normal hover:bg-brand/10',
    className
  );
}

export function AppBar({
  onWorkspacesClick,
  showWorkspacesButton = true,
  isWorkspacesActive,
  onHoverStart,
  onHoverEnd,
  notificationBell,
  onAutomationClick,
  isAutomationActive = false,
  onMailClick,
  isMailActive = false,
  onFrictionClick,
  isFrictionActive = false,
  appVersion,
  updateVersion,
  onUpdateClick,
}: AppBarProps) {
  const sections: AppBarSection[] = [];

  if (showWorkspacesButton) {
    const localItems: AppBarSectionItem[] = [
      {
        key: 'local-workspaces',
        kind: 'icon-button',
        label: 'Workspaces',
        icon: LayoutIcon,
        isActive: isWorkspacesActive,
        onClick: onWorkspacesClick,
      },
    ];
    if (onAutomationClick) {
      localItems.push({
        key: 'automation',
        kind: 'icon-button',
        label: 'Auto Mode',
        icon: RobotIcon,
        isActive: isAutomationActive,
        onClick: onAutomationClick,
      });
    }
    if (onMailClick) {
      localItems.push({
        key: 'mail',
        kind: 'icon-button',
        label: 'Mail',
        icon: EnvelopeIcon,
        isActive: isMailActive,
        onClick: onMailClick,
      });
    }
    if (onFrictionClick) {
      localItems.push({
        key: 'friction',
        kind: 'icon-button',
        label: 'Friction Log',
        icon: WarningIcon,
        isActive: isFrictionActive,
        onClick: onFrictionClick,
      });
    }
    sections.push({ key: 'local', label: 'Local', items: localItems });
  }

  function renderSectionItem(item: AppBarSectionItem): ReactNode {
    return (
      <Tooltip content={item.label} side="right">
        <button
          type="button"
          onClick={item.onClick}
          className={getStandardAppBarButtonClassName({
            isActive: item.isActive,
            className: item.className,
          })}
          aria-label={item.label}
        >
          <item.icon className="size-icon-base" weight="bold" />
        </button>
      </Tooltip>
    );
  }

  return (
    <div
      onMouseEnter={onHoverStart}
      onMouseLeave={onHoverEnd}
      className={cn(
        'flex flex-col items-center h-full min-h-0 overflow-y-auto p-base gap-base',
        'bg-secondary border-r border-border'
      )}
    >
      {sections.map((section) => (
        <div key={section.key} className="flex flex-col items-center gap-1">
          <AppBarSectionLabel>{section.label}</AppBarSectionLabel>
          {section.items.map((item) => (
            <div
              key={item.key}
              className={item.wrapperClassName}
            >
              {renderSectionItem(item)}
            </div>
          ))}
        </div>
      ))}

      {/* Bottom section: Notifications + version. The vibe-kanban /
          BloopAI social links and account popover are removed in the
          local-first single-user build. */}
      <div className="mt-auto pt-base flex flex-col items-center gap-4">
        {notificationBell}
        {updateVersion ? (
          <Tooltip content={`Update to v${updateVersion}`} side="right">
            <button
              type="button"
              onClick={onUpdateClick}
              className={cn(
                'flex items-center justify-center py-1 rounded-md w-10',
                'text-[9px] font-ibm-plex-mono font-medium leading-none',
                'bg-brand text-on-brand hover:bg-brand-hover',
                'transition-colors cursor-pointer'
              )}
            >
              Update
            </button>
          </Tooltip>
        ) : (
          appVersion && (
            <p
              className="text-[9px] font-ibm-plex-mono text-low leading-none truncate max-w-10 text-center"
              title={`v${appVersion}`}
            >
              v{appVersion}
            </p>
          )
        )}
      </div>
    </div>
  );
}
