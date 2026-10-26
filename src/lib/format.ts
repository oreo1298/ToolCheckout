// Display helpers. All timestamps are UTC milliseconds shown in local time.

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

/** "45m", "3h 20m", "2d 4h" — compact, for "how long out". */
export function duration(ms: number): string {
  const abs = Math.max(0, Math.round(ms));
  if (abs < MIN) return '<1m';
  if (abs < HOUR) return `${Math.floor(abs / MIN)}m`;
  if (abs < DAY) {
    const h = Math.floor(abs / HOUR);
    const m = Math.floor((abs % HOUR) / MIN);
    return m ? `${h}h ${m}m` : `${h}h`;
  }
  const d = Math.floor(abs / DAY);
  const h = Math.floor((abs % DAY) / HOUR);
  return h ? `${d}d ${h}h` : `${d}d`;
}

/** "Due in 2h", "3h overdue". */
export function dueLabel(dueAt: number, now: number): string {
  return dueAt >= now ? `due in ${duration(dueAt - now)}` : `${duration(now - dueAt)} overdue`;
}

/** m:ss countdown. */
export function countdown(ms: number): string {
  const s = Math.max(0, Math.ceil(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

function sameDay(a: Date, b: Date) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

export function time(ms: number): string {
  return new Date(ms).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
}

export function date(ms: number): string {
  return new Date(ms).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

/** "2:15 PM" today, "Tue 2:15 PM" this week, else "Mar 3, 2:15 PM". */
export function when(ms: number, now = Date.now()): string {
  const d = new Date(ms);
  const n = new Date(now);
  if (sameDay(d, n)) return time(ms);
  const yesterday = new Date(now - DAY);
  if (sameDay(d, yesterday)) return `Yesterday ${time(ms)}`;
  if (Math.abs(now - ms) < 6 * DAY) {
    return `${d.toLocaleDateString(undefined, { weekday: 'short' })} ${time(ms)}`;
  }
  return d.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' });
}

export function dateTime(ms: number): string {
  return new Date(ms).toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
}

export function money(cents: number | null | undefined, symbol = '$'): string {
  if (cents == null) return '—';
  const v = (cents / 100).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  return `${symbol}${v}`;
}

/** Parses "12.50" / "$12" into cents; null for blank, NaN for junk. */
export function parseMoney(text: string): number | null {
  const t = text.replace(/[^0-9.,-]/g, '').replace(',', '.');
  if (!t.trim()) return null;
  const v = Number(t);
  return Number.isFinite(v) && v >= 0 ? Math.round(v * 100) : NaN;
}

export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return '?';
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

/** Stable hue from a name, for avatar colors. */
export function hue(name: string): number {
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return h % 360;
}

/** Loan period in minutes → "4 hours", "1 day", "90 minutes". */
export function loanPeriod(minutes: number): string {
  if (minutes % (60 * 24) === 0) {
    const d = minutes / (60 * 24);
    return `${d} day${d === 1 ? '' : 's'}`;
  }
  if (minutes % 60 === 0) {
    const h = minutes / 60;
    return `${h} hour${h === 1 ? '' : 's'}`;
  }
  return `${minutes} minutes`;
}

/** Local "today at HH:MM" (or tomorrow if already past) as UTC ms. */
export function todayAt(hhmm: string, now = Date.now()): number {
  const [h, m] = hhmm.split(':').map(Number);
  const d = new Date(now);
  d.setHours(h, m, 0, 0);
  if (d.getTime() <= now) d.setDate(d.getDate() + 1);
  return d.getTime();
}
