import { createFileRoute, useParams } from '@tanstack/react-router';
import { MailThreadView } from '@/pages/mail/MailThreadView';

function MailThreadRoute() {
  const { threadId } = useParams({ strict: false });
  if (!threadId) {
    return null;
  }
  return <MailThreadView threadId={threadId} />;
}

export const Route = createFileRoute('/_app/mail/threads/$threadId')({
  component: MailThreadRoute,
});
