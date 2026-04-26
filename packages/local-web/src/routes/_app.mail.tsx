import { createFileRoute } from '@tanstack/react-router';
import { MailInboxPage } from '@/pages/mail/MailInboxPage';

export const Route = createFileRoute('/_app/mail')({
  component: () => <MailInboxPage />,
});
