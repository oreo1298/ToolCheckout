// App-wide reactive state: bootstrap info, tech session, navigation, toasts,
// and the "ask for a technician PIN" flow.
import { listen } from '@tauri-apps/api/event';
import {
  api,
  asApiError,
  type AppStatus,
  type AwaitingItem,
  type Bootstrap,
  type BorrowerPick,
  type LoanRow,
  type TechSession,
  type ToolRow,
} from './api';
import { theme } from './themeBoot';

export type Screen = 'home' | 'tools' | 'borrowers' | 'incidents' | 'reports' | 'settings';

export interface Toast {
  id: number;
  kind: 'success' | 'error' | 'info';
  text: string;
}

/** The full-screen task sheet currently open, if any. */
export type Flow =
  | { kind: 'checkout'; tools: ToolRow[]; borrower: BorrowerPick | null }
  | { kind: 'return'; loans: LoanRow[] }
  | { kind: 'inspect'; item: AwaitingItem };

interface UnlockRequest {
  reason?: string;
  resolve: (ok: boolean) => void;
}

const TOUCH_EVERY_MS = 5_000;

class AppStore {
  boot = $state<Bootstrap | null>(null);
  status = $state<AppStatus | null>(null);
  session = $state<TechSession | null>(null);
  screen = $state<Screen>('home');
  /** Bumped whenever the backend reports a data change; views re-fetch on it. */
  dataVersion = $state(0);
  /** Ticks every second for countdowns and "how long" labels. */
  nowMs = $state(Date.now());
  unlockRequest = $state<UnlockRequest | null>(null);
  flow = $state<Flow | null>(null);
  /** Tool whose detail panel is open (from any screen). */
  toolDetailId = $state<number | null>(null);
  /** True from a not-set-up start until the recovery code is acknowledged. */
  onboarding = $state(false);
  toasts = $state<Toast[]>([]);

  private clockOffset = 0;
  private lastTouch = 0;
  private toastSeq = 0;

  get settings() {
    return this.status?.settings ?? null;
  }

  now() {
    return Date.now() + this.clockOffset;
  }

  async init() {
    this.boot = await api.bootstrap();
    if (this.boot.status) this.applyStatus(this.boot.status);
    this.onboarding = this.boot.status?.setUp === false;
    await listen('data-changed', () => {
      this.dataVersion++;
      void this.refreshStatus();
    });
    // Overdue / end-of-day reminders also appear in the app, in case OS
    // notifications are switched off.
    await listen<{ title: string }>('reminder', (e) => this.toast(e.payload.title, 'info'));
    setInterval(() => this.tick(), 1000);
    const onActivity = () => this.activity();
    window.addEventListener('pointerdown', onActivity, { passive: true });
    window.addEventListener('keydown', onActivity, { passive: true });
  }

  applyStatus(s: AppStatus) {
    this.status = s;
    this.session = s.session;
    this.clockOffset = s.now - Date.now();
    theme.setOverride(s.settings.theme);
  }

  async refreshStatus() {
    try {
      this.applyStatus(await api.status());
    } catch {
      /* startup error already shown */
    }
  }

  private tick() {
    this.nowMs = this.now();
    if (this.session && this.nowMs >= this.session.expiresAt) {
      // The backend has locked too; confirm and update the badge.
      void api.session().then((s) => (this.session = s));
      this.session = null;
    }
  }

  /** Keeps tech mode unlocked while someone is using the app. */
  activity() {
    if (!this.session) return;
    const now = Date.now();
    if (now - this.lastTouch < TOUCH_EVERY_MS) return;
    this.lastTouch = now;
    void api.touch().then((s) => (this.session = s));
  }

  async lock() {
    await api.lock();
    this.session = null;
  }

  /** Opens the unlock dialog; resolves true once a technician unlocked. */
  requestUnlock(reason?: string): Promise<boolean> {
    if (this.session) return Promise.resolve(true);
    this.unlockRequest?.resolve(false);
    return new Promise((resolve) => {
      this.unlockRequest = { reason, resolve };
    });
  }

  finishUnlock(session: TechSession | null) {
    if (session) this.session = session;
    this.unlockRequest?.resolve(!!session);
    this.unlockRequest = null;
  }

  /**
   * Runs a technician-only action, asking for a PIN first if needed and once
   * more if the session timed out mid-way. Returns undefined if cancelled.
   */
  async withTech<T>(fn: () => Promise<T>, reason?: string): Promise<T | undefined> {
    for (let attempt = 0; attempt < 2; attempt++) {
      if (!this.session && !(await this.requestUnlock(reason))) return undefined;
      try {
        return await fn();
      } catch (e) {
        const err = asApiError(e);
        if (err.code !== 'tech_required') throw err;
        this.session = null;
      }
    }
    return undefined;
  }

  toast(text: string, kind: Toast['kind'] = 'success') {
    const id = ++this.toastSeq;
    this.toasts = [...this.toasts, { id, kind, text }];
    setTimeout(() => this.dismissToast(id), kind === 'error' ? 7000 : 3500);
  }

  error(e: unknown) {
    this.toast(asApiError(e).message, 'error');
  }

  dismissToast(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }
}

export const app = new AppStore();
