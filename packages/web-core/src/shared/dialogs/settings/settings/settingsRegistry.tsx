import {
  GearIcon,
  GitBranchIcon,
  CpuIcon,
  PlugIcon,
} from '@phosphor-icons/react';
import type { Icon } from '@phosphor-icons/react';
import { GeneralSettingsSection } from './GeneralSettingsSection';
import { ReposSettingsSection } from './ReposSettingsSection';
import { AgentsSettingsSection } from './AgentsSettingsSection';
import { McpSettingsSection } from './McpSettingsSection';

export type SettingsSectionType =
  // TODO(local-first): 'organizations' | 'relay' | 'remote-projects' are kept
  // as accepted ids so legacy callers compile, but are routed to 'general'.
  | 'general'
  | 'repos'
  | 'agents'
  | 'mcp'
  | 'organizations'
  | 'relay'
  | 'remote-projects';

export type SettingsSectionGroup = 'host' | 'universal';

export type SettingsSectionInitialState = {
  general: undefined;
  repos: { repoId?: string } | undefined;
  agents: { executor?: string; variant?: string } | undefined;
  mcp: undefined;
  organizations: undefined;
  relay: { hostId?: string } | undefined;
  'remote-projects': { organizationId?: string } | undefined;
};

export interface SettingsSectionDefinition {
  id: SettingsSectionType;
  icon: Icon;
  group: SettingsSectionGroup;
}

export const SETTINGS_SECTION_DEFINITIONS: SettingsSectionDefinition[] = [
  { id: 'general', icon: GearIcon, group: 'host' },
  { id: 'repos', icon: GitBranchIcon, group: 'host' },
  { id: 'agents', icon: CpuIcon, group: 'host' },
  { id: 'mcp', icon: PlugIcon, group: 'host' },
];

export function isHostSpecificSettingsSection(
  type: SettingsSectionType
): boolean {
  return (
    SETTINGS_SECTION_DEFINITIONS.find((section) => section.id === type)
      ?.group === 'host'
  );
}

export function renderSettingsSection(
  type: SettingsSectionType,
  initialState?: SettingsSectionInitialState[SettingsSectionType],
  _onClose?: () => void
) {
  switch (type) {
    case 'general':
      return <GeneralSettingsSection />;
    case 'repos':
      return (
        <ReposSettingsSection
          initialState={initialState as SettingsSectionInitialState['repos']}
        />
      );
    case 'agents':
      return <AgentsSettingsSection />;
    case 'mcp':
      return <McpSettingsSection />;
    // TODO(local-first): legacy section ids fall back to general.
    case 'organizations':
    case 'relay':
    case 'remote-projects':
    default:
      return <GeneralSettingsSection />;
  }
}
