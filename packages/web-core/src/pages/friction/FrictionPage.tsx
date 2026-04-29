import { useFrictionSnapshot } from '@/shared/hooks/useFriction';
import type {
  FrictionHeader,
  FrictionSnapshot,
  LatestEntry,
  VentureCard,
} from 'shared/types';

// D-AUTO-8: locked Tailwind tokens. Do not adjust without updating the
// design doc — these are referenced from the spec for cross-tool parity
// (CLI, Python summarizer, dashboard).
const VENTURE_CHIP_CLASS: Record<string, string> = {
  'chief-of-staff': 'bg-indigo-500 text-indigo-50',
  carbonv3: 'bg-emerald-500 text-emerald-50',
  fultech: 'bg-violet-500 text-violet-50',
  'port-analytics': 'bg-sky-500 text-sky-50',
  other: 'bg-zinc-500 text-zinc-50',
};

function ventureChipClass(venture: string): string {
  return VENTURE_CHIP_CLASS[venture] ?? 'bg-zinc-500 text-zinc-50';
}

function formatRelativeMinutes(iso: string | null): string {
  if (!iso) return 'never';
  const ms = Date.now() - new Date(iso).getTime();
  if (Number.isNaN(ms) || ms < 0) return 'just now';
  const min = Math.floor(ms / 60_000);
  if (min < 1) return 'just now';
  if (min < 60) return `${min} min ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  return `${day}d ago`;
}

function formatHoursToDay3(hours: number): string {
  if (hours <= 0) return 'past day-3 cutoff';
  if (hours < 1) return `${Math.round(hours * 60)} min to day 3`;
  if (hours < 48) return `${Math.round(hours)}h to day 3`;
  return `${Math.round(hours / 24)}d to day 3`;
}

function SeverityChip({
  label,
  count,
  tone,
}: {
  label: string;
  count: number;
  tone: 'p1' | 'p2' | 'p3';
}) {
  const cls =
    tone === 'p1'
      ? 'bg-rose-500 text-rose-50'
      : tone === 'p2'
        ? 'bg-amber-500 text-amber-50'
        : 'bg-zinc-400 text-zinc-50';
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium ${cls}`}
      title={`${label}: ${count}`}
    >
      <span className="font-semibold">{label}</span>
      <span className="opacity-90">{count}</span>
    </span>
  );
}

function HeaderStrip({ header }: { header: FrictionHeader }) {
  const observationMode = header.day_n > 0 && header.day_n <= 3;
  return (
    <header className="flex flex-col gap-3 rounded-md border border-secondary bg-panel p-4">
      <div className="flex flex-wrap items-baseline gap-x-6 gap-y-2">
        <div className="flex items-baseline gap-2">
          <span className="text-lg font-semibold text-high">
            Day {header.day_n} of 3
          </span>
          <span className="text-base text-low">
            {formatHoursToDay3(header.hours_to_day_3)}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <SeverityChip label="P1" count={header.p1_count} tone="p1" />
          <SeverityChip label="P2" count={header.p2_count} tone="p2" />
          <SeverityChip label="P3" count={header.p3_count} tone="p3" />
        </div>
        <div className="flex items-center gap-2 text-base text-low">
          <span>{header.event_count} events</span>
        </div>
        <div className="ml-auto flex flex-col items-end text-base text-low">
          <span>Last entry: {formatRelativeMinutes(header.last_entry_ts)}</span>
          <span>Last event: {formatRelativeMinutes(header.last_event_ts)}</span>
        </div>
      </div>
      {observationMode && (
        <div
          role="status"
          className="rounded-sm border border-amber-500/50 bg-amber-500/10 px-3 py-2 text-base text-amber-100"
        >
          Observation mode — log friction, do not fix unless an off-ramp
          trigger fires (supervisor crash, app boot fail, can&apos;t create
          workspace, can&apos;t dispatch).
        </div>
      )}
    </header>
  );
}

function Day3Banner({ status }: { status: string }) {
  return (
    <div
      role="status"
      className="flex flex-col gap-1 rounded-md border border-emerald-500/60 bg-emerald-500/10 p-4"
    >
      <span className="text-base font-medium text-emerald-100">
        Day 3 candidates ready
      </span>
      <span className="text-base text-emerald-200/80 break-all">
        {status}
      </span>
      <span className="text-xs text-emerald-200/60">
        Open the file above to review Phase 14 candidate picks.
      </span>
    </div>
  );
}

function SkippedLinesBanner({ count }: { count: number }) {
  return (
    <div
      role="alert"
      className="flex flex-col gap-1 rounded-md border border-amber-500/60 bg-amber-500/10 p-3"
    >
      <span className="text-base font-medium text-amber-100">
        Skipped {count} malformed line{count === 1 ? '' : 's'}
      </span>
      <span className="text-xs text-amber-200/80">
        The reader skipped JSONL records it could not parse. Inspect
        friction-log.jsonl / events.jsonl before day-3 reread.
      </span>
    </div>
  );
}

function LatestEntryRow({ entry }: { entry: LatestEntry }) {
  return (
    <div className="flex flex-col gap-0.5 rounded-sm border border-secondary bg-secondary/40 p-2 text-base">
      <div className="flex items-center gap-2 text-low">
        <span className="font-mono text-xs">
          {formatRelativeMinutes(entry.ts)}
        </span>
        <span className="rounded-full bg-secondary px-2 text-xs">
          {entry.layer}
        </span>
        <span className="rounded-full bg-secondary px-2 text-xs">
          {entry.severity}
        </span>
      </div>
      <span className="text-high break-words">{entry.what_blocked}</span>
    </div>
  );
}

function VentureCardView({ card }: { card: VentureCard }) {
  return (
    <article className="flex flex-col gap-3 rounded-md border border-secondary bg-panel p-4">
      <div className="flex flex-wrap items-center gap-3">
        <span
          className={`rounded-full px-3 py-1 text-base font-medium ${ventureChipClass(card.venture)}`}
        >
          {card.venture}
        </span>
        <span className="text-base text-low">
          score {card.weighted_score.toFixed(1)}
        </span>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <SeverityChip label="P1" count={card.p1_count} tone="p1" />
        <SeverityChip label="P2" count={card.p2_count} tone="p2" />
        <SeverityChip label="P3" count={card.p3_count} tone="p3" />
        <span className="ml-auto text-base text-low">
          {card.event_count} events ·{' '}
          {Number(card.time_lost_min_total).toString()} min lost
        </span>
      </div>
      {card.latest_entry ? (
        <LatestEntryRow entry={card.latest_entry} />
      ) : (
        <span className="text-base text-low italic">
          No manual entries yet — only auto events.
        </span>
      )}
    </article>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center gap-2 rounded-md border border-dashed border-secondary bg-panel p-10 text-center">
      <span className="text-lg text-high">No friction logged yet</span>
      <span className="text-base text-low max-w-md">
        Run your first dispatch in any venture, then come back. The
        dashboard slices entries by venture and ranks by P1*30 + P2*10 +
        P3*1 + events*1.0 weighting.
      </span>
    </div>
  );
}

function LoadingSkeleton() {
  return (
    <div className="flex flex-col gap-3" aria-busy="true">
      <div className="h-24 animate-pulse rounded-md border border-secondary bg-panel" />
      <div className="grid gap-3 md:grid-cols-2">
        {[0, 1, 2, 3].map((i) => (
          <div
            key={i}
            className="h-40 animate-pulse rounded-md border border-secondary bg-panel"
          />
        ))}
      </div>
    </div>
  );
}

function ErrorBanner({ message }: { message: string }) {
  return (
    <div
      role="alert"
      className="rounded-md border border-rose-500/60 bg-rose-500/10 p-4 text-base text-rose-100"
    >
      Couldn&apos;t load friction snapshot: {message}
    </div>
  );
}

function FrictionContent({ data }: { data: FrictionSnapshot }) {
  const sortedVentures = [...data.ventures].sort(
    (a, b) => b.weighted_score - a.weighted_score
  );
  return (
    <div className="flex flex-col gap-4">
      {data.day_3_status && <Day3Banner status={data.day_3_status} />}
      <HeaderStrip header={data.header} />
      {data.skipped_lines > 0 && (
        <SkippedLinesBanner count={data.skipped_lines} />
      )}
      {sortedVentures.length === 0 ? (
        <EmptyState />
      ) : (
        <section className="grid gap-3 md:grid-cols-2">
          {sortedVentures.map((card) => (
            <VentureCardView key={card.venture} card={card} />
          ))}
        </section>
      )}
    </div>
  );
}

export function FrictionPage() {
  const { data, isLoading, error } = useFrictionSnapshot();
  return (
    <div className="flex h-full flex-1 flex-col gap-4 overflow-y-auto bg-primary p-6">
      <div className="flex flex-col gap-1">
        <h1 className="text-lg text-high">Friction log</h1>
        <span className="text-base text-low">
          Per-venture friction & event counts, refreshed every 30s. See
          docs/designs/friction-log-discipline.md for the discipline rules.
        </span>
      </div>
      {isLoading && !data && <LoadingSkeleton />}
      {error && (
        <ErrorBanner
          message={error instanceof Error ? error.message : 'unknown error'}
        />
      )}
      {data && <FrictionContent data={data} />}
    </div>
  );
}
