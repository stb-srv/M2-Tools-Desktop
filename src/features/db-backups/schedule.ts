// Interval-based schedule ("every N hours") rather than an exact clock time -
// simpler and more robust than time-of-day matching, since it works
// correctly regardless of whether the app happens to be open at the exact
// target minute. There is no bundled background service (see CrashWatch.tsx,
// the app's only periodic poller): this can only ever mean "check on each
// tick whether enough time has passed since the last auto-backup, and if so
// run one" - it does nothing while the app is closed, unlike a real cron.

/** Selectable schedule intervals; 0 means "disabled". */
export const SCHEDULE_HOURS_OPTIONS = [0, 6, 12, 24, 48] as const;

/**
 * True if a new automatic backup is due: scheduling is enabled
 * (`scheduleHours > 0`) and either no automatic backup has ever run yet, or
 * enough time has elapsed since the last one.
 */
export function isBackupDue(nowMs: number, lastAutoAtIso: string | null, scheduleHours: number): boolean {
  if (!scheduleHours || scheduleHours <= 0) return false;
  if (!lastAutoAtIso) return true;
  const lastMs = Date.parse(lastAutoAtIso);
  if (Number.isNaN(lastMs)) return true;
  return nowMs - lastMs >= scheduleHours * 60 * 60 * 1000;
}

/** Next due timestamp, or null if scheduling is disabled or the timestamp is unparseable. */
export function nextDueAt(lastAutoAtIso: string | null, scheduleHours: number): Date | null {
  if (!scheduleHours || scheduleHours <= 0) return null;
  if (!lastAutoAtIso) return new Date();
  const lastMs = Date.parse(lastAutoAtIso);
  if (Number.isNaN(lastMs)) return new Date();
  return new Date(lastMs + scheduleHours * 60 * 60 * 1000);
}
