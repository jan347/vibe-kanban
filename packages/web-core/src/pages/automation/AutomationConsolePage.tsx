import { useEffect, useMemo, useState } from 'react';
import {
  CheckCircleIcon,
  ClockCounterClockwiseIcon,
  ProhibitIcon,
  RobotIcon,
  ShieldCheckIcon,
} from '@phosphor-icons/react';
import {
  useAutoApprovalLog,
  useAutomations,
  useFireAutomation,
  useResolveAutoApproval,
  useSafetyConfig,
  useUpdateAutomation,
  useUpdateSafetyConfig,
} from '@/shared/hooks/useAutomations';
import type {
  AutoApprovalLogEntry,
  AutomationRule,
  UpdateAutomationRule,
  UpdateSafetyConfig,
} from 'shared/types';

const EMPTY_AUTOMATION_PATCH: UpdateAutomationRule = {
  name: null,
  trigger_kind: null,
  trigger_config: null,
  prompt_template_id: null,
  model_preset_id: null,
  prompt_override: null,
  enabled: null,
};

const EMPTY_SAFETY_PATCH: UpdateSafetyConfig = {
  require_human_approval: null,
  max_concurrent_dispatch: null,
  max_daily_dispatch: null,
  cooldown_seconds: null,
  auto_approval_enabled: null,
  auto_approval_policy: null,
  auto_approval_model_preset_id: null,
};

function formatRelative(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  const sec = Math.max(0, Math.floor(ms / 1000));
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return new Date(iso).toLocaleDateString();
}

function PendingRow({ item }: { item: AutoApprovalLogEntry }) {
  const resolve = useResolveAutoApproval();
  const busy = resolve.isPending;
  return (
    <li className="flex items-start gap-3 rounded-sm border border-secondary bg-panel p-3">
      <ClockCounterClockwiseIcon className="size-4 shrink-0 mt-1 text-low" />
      <div className="flex flex-1 flex-col gap-1 min-w-0">
        <div className="flex items-baseline gap-2">
          <span className="rounded-full bg-secondary px-2 text-base text-low">
            {item.action_kind}
          </span>
          <span className="text-base text-high truncate">
            {item.action_summary}
          </span>
          <span className="text-low shrink-0 ml-auto">
            {formatRelative(item.decided_at)}
          </span>
        </div>
        {item.reasoning && (
          <span className="text-low text-base">{item.reasoning}</span>
        )}
        <div className="mt-1 flex gap-2">
          <button
            disabled={busy}
            onClick={() =>
              resolve.mutate({ id: item.id, decision: 'approved' })
            }
            className="rounded-sm bg-success px-3 py-1 text-base text-high disabled:opacity-50"
          >
            Approve
          </button>
          <button
            disabled={busy}
            onClick={() => resolve.mutate({ id: item.id, decision: 'denied' })}
            className="rounded-sm bg-error px-3 py-1 text-base text-high disabled:opacity-50"
          >
            Deny
          </button>
        </div>
      </div>
    </li>
  );
}

function AutomationRow({ rule }: { rule: AutomationRule }) {
  const update = useUpdateAutomation();
  const fire = useFireAutomation();
  const busy = update.isPending || fire.isPending;
  return (
    <li className="flex items-center gap-3 rounded-sm border border-secondary bg-panel p-3">
      <div className="flex flex-1 flex-col min-w-0">
        <div className="flex items-baseline gap-2">
          <span className="text-base text-high truncate">{rule.name}</span>
          <span className="rounded-full bg-secondary px-2 text-base text-low">
            {rule.trigger_kind}
          </span>
          {!rule.enabled && (
            <span className="rounded-full bg-error px-2 text-base text-high">
              disabled
            </span>
          )}
        </div>
        <span className="text-low text-base">
          {rule.last_fired_at
            ? `last fired ${formatRelative(rule.last_fired_at)}`
            : 'never fired'}
        </span>
      </div>
      <button
        disabled={busy}
        onClick={() =>
          update.mutate({
            id: rule.id,
            patch: { ...EMPTY_AUTOMATION_PATCH, enabled: !rule.enabled },
          })
        }
        className="rounded-sm border border-secondary px-3 py-1 text-base text-high hover:bg-secondary disabled:opacity-50"
      >
        {rule.enabled ? 'Disable' : 'Enable'}
      </button>
      <button
        disabled={busy || !rule.enabled}
        onClick={() => fire.mutate(rule.id)}
        className="rounded-sm bg-accent px-3 py-1 text-base text-high disabled:opacity-50"
      >
        Fire now
      </button>
    </li>
  );
}

export function AutomationConsolePage() {
  const config = useSafetyConfig();
  const updateConfig = useUpdateSafetyConfig();
  const pending = useAutoApprovalLog({ pendingOnly: true });
  const recent = useAutoApprovalLog({});
  const automations = useAutomations();

  const [policyDraft, setPolicyDraft] = useState<string>('');
  useEffect(() => {
    if (config.data?.auto_approval_policy != null) {
      setPolicyDraft(config.data.auto_approval_policy);
    } else if (config.data) {
      setPolicyDraft('');
    }
  }, [config.data]);

  const stats = useMemo(() => {
    const all = recent.data ?? [];
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const since = today.getTime();
    let approved = 0;
    let denied = 0;
    let escalated = 0;
    for (const e of all) {
      if (new Date(e.decided_at).getTime() < since) continue;
      if (e.decision === 'approved') approved++;
      else if (e.decision === 'denied') denied++;
      else if (e.decision === 'escalated') escalated++;
    }
    return { approved, denied, escalated };
  }, [recent.data]);

  const enabled = config.data?.auto_approval_enabled ?? false;

  return (
    <div className="flex h-full flex-1 flex-col gap-6 overflow-y-auto bg-primary p-6">
      <header className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-3">
          <RobotIcon className="size-6 text-high" />
          <div className="flex flex-col">
            <h1 className="text-lg text-high">Auto mode</h1>
            <span className="text-low text-base">
              Agents handle routine work; only escalations reach you.
            </span>
          </div>
        </div>
        <button
          disabled={!config.data || updateConfig.isPending}
          onClick={() =>
            updateConfig.mutate({
              ...EMPTY_SAFETY_PATCH,
              auto_approval_enabled: !enabled,
            })
          }
          className={`rounded-sm px-4 py-2 text-base text-high disabled:opacity-50 ${
            enabled ? 'bg-success' : 'bg-secondary'
          }`}
        >
          {enabled ? 'Auto mode: ON' : 'Auto mode: OFF'}
        </button>
      </header>

      <section className="grid grid-cols-3 gap-3">
        <div className="rounded-sm border border-secondary bg-panel p-3">
          <div className="flex items-center gap-2 text-low">
            <CheckCircleIcon className="size-4" /> Approved today
          </div>
          <div className="text-lg text-high">{stats.approved}</div>
        </div>
        <div className="rounded-sm border border-secondary bg-panel p-3">
          <div className="flex items-center gap-2 text-low">
            <ProhibitIcon className="size-4" /> Denied today
          </div>
          <div className="text-lg text-high">{stats.denied}</div>
        </div>
        <div className="rounded-sm border border-secondary bg-panel p-3">
          <div className="flex items-center gap-2 text-low">
            <ClockCounterClockwiseIcon className="size-4" /> Pending
          </div>
          <div className="text-lg text-high">{pending.data?.length ?? 0}</div>
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="text-base text-high">
          Pending approvals{' '}
          <span className="text-low">({pending.data?.length ?? 0})</span>
        </h2>
        {pending.isLoading && <span className="text-low">Loading…</span>}
        {pending.error && (
          <span className="text-low">
            Couldn't load:{' '}
            {pending.error instanceof Error ? pending.error.message : 'unknown'}
          </span>
        )}
        {pending.data && pending.data.length === 0 && (
          <span className="text-low">No pending approvals — all clear.</span>
        )}
        <ul className="flex flex-col gap-2">
          {pending.data?.map((item) => (
            <PendingRow key={item.id} item={item} />
          ))}
        </ul>
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="flex items-center gap-2 text-base text-high">
          <ShieldCheckIcon className="size-4" /> Policy
        </h2>
        <p className="text-low text-base">
          One rule per line. <code>allow: pattern</code> auto-approves matches;{' '}
          <code>deny: pattern</code> denies them. Patterns are matched as
          contiguous whitespace-separated tokens — so <code>deny: rm</code>{' '}
          matches <code>rm -rf /tmp</code> but not <code>farm</code>. Anything
          that doesn't match escalates to you.
        </p>
        <textarea
          value={policyDraft}
          onChange={(e) => setPolicyDraft(e.target.value)}
          rows={8}
          spellCheck={false}
          className="rounded-sm border border-secondary bg-panel p-3 text-base text-high font-mono"
          placeholder={`allow: run tests\nallow: read file\ndeny: rm -rf\ndeny: drop table`}
        />
        <div className="flex gap-2">
          <button
            disabled={updateConfig.isPending || !config.data}
            onClick={() =>
              updateConfig.mutate({
                ...EMPTY_SAFETY_PATCH,
                auto_approval_policy: policyDraft,
              })
            }
            className="rounded-sm bg-brand px-4 py-2 text-sm font-medium text-on-brand transition-colors hover:bg-brand-hover disabled:opacity-50"
          >
            {updateConfig.isPending ? 'Saving…' : 'Save policy'}
          </button>
          {config.data?.auto_approval_policy != null && (
            <button
              disabled={updateConfig.isPending}
              onClick={() =>
                setPolicyDraft(config.data?.auto_approval_policy ?? '')
              }
              className="rounded-sm border border-secondary px-4 py-2 text-base text-high hover:bg-secondary"
            >
              Reset
            </button>
          )}
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="text-base text-high">
          Automations{' '}
          <span className="text-low">({automations.data?.length ?? 0})</span>
        </h2>
        {automations.data && automations.data.length === 0 && (
          <span className="text-low">
            No automation rules yet. Create one to schedule recurring
            dispatches.
          </span>
        )}
        <ul className="flex flex-col gap-2">
          {automations.data?.map((rule) => (
            <AutomationRow key={rule.id} rule={rule} />
          ))}
        </ul>
      </section>
    </div>
  );
}
