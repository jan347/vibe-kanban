/**
 * Friction-log discipline (D-AUTO-4): venture tag for workspace creation.
 *
 * Drives the per-venture slicing in `/friction` and the day-3 summarizer.
 * The four known ventures are locked across CLI, dashboard, and Python
 * summarizer — mirror this list when adding a new one.
 */

export const KNOWN_VENTURES = [
  'chief-of-staff',
  'carbonv3',
  'fultech',
  'port-analytics',
] as const;

export type KnownVenture = (typeof KNOWN_VENTURES)[number];

/**
 * `null` is the "(none)" pick: only valid on edit, not on create.
 * `'other'` is the free-text escape hatch — the actual venture string
 * comes from a separate text input.
 */
export type VentureSelection = KnownVenture | 'other' | null;

const STORAGE_KEY = 'gencap.lastVenture';

export function loadLastVenture(): VentureSelection {
  if (typeof window === 'undefined') return null;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    if (raw === 'other') return 'other';
    if ((KNOWN_VENTURES as readonly string[]).includes(raw)) {
      return raw as KnownVenture;
    }
    return null;
  } catch {
    // Storage may be disabled (Safari private mode, etc.). Fall through.
    return null;
  }
}

export function saveLastVenture(selection: VentureSelection): void {
  if (typeof window === 'undefined') return;
  try {
    if (selection === null) {
      window.localStorage.removeItem(STORAGE_KEY);
    } else {
      window.localStorage.setItem(STORAGE_KEY, selection);
    }
  } catch {
    // Best-effort; silently ignore quota / disabled storage.
  }
}
