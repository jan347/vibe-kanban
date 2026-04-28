import { useState } from 'react';
import { AppBarUserPopover } from '@gencap/ui/components/AppBarUserPopover';
import { SettingsDialog } from '@/shared/dialogs/settings/SettingsDialog';
import { useAuth } from '@/shared/hooks/auth/useAuth';
import { useActions } from '@/shared/hooks/useActions';
import { Actions } from '@/shared/actions';

// TODO(local-first): orgs are dead, but the AppBarUserPopover prop shape still
// expects an organizations array. We pass an empty list and ignore selection.
interface AppBarOrganization {
  id: string;
  name: string;
  is_personal: boolean;
}

interface AppBarUserPopoverContainerProps {
  organizations: AppBarOrganization[];
  selectedOrgId: string;
  onOrgSelect: (orgId: string) => void;
}

export function AppBarUserPopoverContainer({
  organizations,
  selectedOrgId,
  onOrgSelect,
}: AppBarUserPopoverContainerProps) {
  const { executeAction } = useActions();
  const { isSignedIn } = useAuth();
  const [open, setOpen] = useState(false);
  const [avatarError, setAvatarError] = useState(false);

  // TODO(local-first): no remote profile/avatar in single-user mode.
  const avatarUrl: string | null = null;

  const handleSignIn = async () => {
    await executeAction(Actions.SignIn);
  };

  const handleLogout = async () => {
    await executeAction(Actions.SignOut);
  };

  const handleOrgSettings = async (_orgId: string) => {
    await SettingsDialog.show({ initialSection: 'general' });
  };

  const handleSettings = async () => {
    setOpen(false);
    await SettingsDialog.show();
  };

  return (
    <AppBarUserPopover
      isSignedIn={isSignedIn}
      avatarUrl={avatarUrl}
      avatarError={avatarError}
      organizations={organizations as never}
      selectedOrgId={selectedOrgId}
      open={open}
      onOpenChange={setOpen}
      onOrgSelect={onOrgSelect}
      onOrgSettings={handleOrgSettings}
      onSignIn={handleSignIn}
      onLogout={handleLogout}
      onAvatarError={() => setAvatarError(true)}
      onSettings={handleSettings}
    />
  );
}
