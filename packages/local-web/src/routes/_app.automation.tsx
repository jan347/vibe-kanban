import { createFileRoute } from '@tanstack/react-router';
import { AutomationConsolePage } from '@/pages/automation/AutomationConsolePage';

export const Route = createFileRoute('/_app/automation' as never)({
  component: () => <AutomationConsolePage />,
});
