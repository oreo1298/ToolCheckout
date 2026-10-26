// Human-readable labels for enums and audit actions.
import { dateTime } from './format';

export const AUDIT_ACTIONS: Record<string, string> = {
  'setup.complete': 'Shop set up',
  'tech.unlock': 'Unlocked tech mode',
  'technician.create': 'Added technician',
  'technician.update': 'Updated technician',
  'technician.pin_reset': 'Reset technician PIN',
  'recovery.generate': 'New recovery code',
  'recovery.pin_reset': 'PIN reset with recovery code',
  'pin.lockout': 'PIN lockout',
  'settings.update': 'Changed settings',
  'tool.create': 'Added tool',
  'tool.update': 'Edited tool',
  'tool.condition': 'Updated condition',
  'tool.status': 'Changed status',
  'tool.retire': 'Retired tool',
  'tool.reinstate': 'Reinstated tool',
  'tool.repair_out': 'Sent to repair',
  'tool.repair_back': 'Back from repair',
  'tool.lost': 'Marked lost',
  'tool.found': 'Marked found',
  'borrower.create': 'Added borrower',
  'borrower.update': 'Edited borrower',
  'borrower.pin_reset': 'Reset borrower PIN',
  'checkout.create': 'Checked out',
  'checkout.problem_report': 'Reported a problem',
  'checkout.dropoff': 'Dropped off for inspection',
  'checkout.inspect': 'Inspected return',
  'inspection.complete': 'Inspected tool',
  'incident.create': 'Opened incident',
  'incident.resolve': 'Resolved incident',
  'backup.create': 'Made a backup',
  'backup.restore': 'Restored a backup',
  'data.relocate': 'Moved data folder',
  'demo.seed': 'Loaded demo data',
};

export function auditLabel(action: string): string {
  return AUDIT_ACTIONS[action] ?? action;
}

/** Compact "key: old → new" summary of an audit entry's before/after JSON. */
export function auditDiff(beforeJson: string | null, afterJson: string | null): string {
  const parse = (s: string | null): Record<string, unknown> => {
    if (!s) return {};
    try {
      const v = JSON.parse(s);
      return v && typeof v === 'object' && !Array.isArray(v) ? v : { value: v };
    } catch {
      return { value: s };
    }
  };
  const b = parse(beforeJson);
  const a = parse(afterJson);
  const keys = [...new Set([...Object.keys(b), ...Object.keys(a)])];
  const fmtVal = (v: unknown) => (v === undefined ? '∅' : typeof v === 'string' ? v : JSON.stringify(v));
  // Timestamps (e.g. lockedUntil, dueAt) read better as dates.
  const isTime = (k: string, v: unknown) => /(At|Until)$/.test(k) && typeof v === 'number' && v > 1e12;
  const fmt = (k: string, v: unknown) => (isTime(k, v) ? dateTime(v as number) : fmtVal(v));
  return keys
    .map((k) => {
      if (!(k in b)) return `${k}: ${fmt(k, a[k])}`;
      if (!(k in a)) return `${k}: ${fmt(k, b[k])} → ∅`;
      return `${k}: ${fmt(k, b[k])} → ${fmt(k, a[k])}`;
    })
    .join(' · ');
}
