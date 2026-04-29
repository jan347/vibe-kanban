import { createFileRoute } from '@tanstack/react-router';
import { FrictionPage } from '@/pages/friction/FrictionPage';

export const Route = createFileRoute('/_app/friction' as never)({
  component: () => <FrictionPage />,
});
