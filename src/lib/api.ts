// Typed wrappers around the Rust commands. Types mirror the serde structs in
// crates/core (camelCase).
import { invoke } from '@tauri-apps/api/core';

export interface ApiError {
  code: string;
  message: string;
  attemptsLeft?: number;
  lockedUntil?: number;
}

export function asApiError(e: unknown): ApiError {
  if (e && typeof e === 'object' && 'code' in e && 'message' in e) return e as ApiError;
  return { code: 'unknown', message: e instanceof Error ? e.message : String(e) };
}

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw asApiError(e);
  }
}

// ------------------------------------------------------------------ types

export type ThemeSetting = 'system' | 'light' | 'dark';

export interface Settings {
  shopName: string;
  defaultLoanMinutes: number;
  borrowerPinRequired: boolean;
  techPinForCheckout: boolean;
  autoLockSeconds: number;
  theme: ThemeSetting;
  overdueNotifications: boolean;
  overdueReminderMinutes: number;
  endOfDayReminder: boolean;
  closingTime: string;
  autoBackup: boolean;
  backupDir: string | null;
  backupKeep: number;
  currencySymbol: string;
  labelStock: string;
}

export interface TechSession {
  techId: number;
  techName: string;
  unlockedAt: number;
  lastActivity: number;
  expiresAt: number;
}

export interface AppStatus {
  setUp: boolean;
  demo: boolean;
  settings: Settings;
  session: TechSession | null;
  schemaVersion: number;
  dbPath: string | null;
  now: number;
}

export interface DataLocation {
  dir: string;
  mode: 'default' | 'custom' | 'portable' | 'environment';
  defaultDir: string;
}

export interface Bootstrap {
  status: AppStatus | null;
  location: DataLocation | null;
  startupError: string | null;
  platform: string;
  version: string;
}

export interface Technician {
  id: number;
  name: string;
  active: boolean;
  createdAt: number;
}

export interface SetupResult {
  technician: Technician;
  recoveryCode: string;
  session: TechSession;
}

export interface AuditRow {
  id: number;
  ts: number;
  actorKind: 'technician' | 'borrower' | 'system';
  actorId: number | null;
  actorName: string;
  action: string;
  entityKind: string;
  entityId: number | null;
  beforeJson: string | null;
  afterJson: string | null;
  prevHash: string;
  hash: string;
}

export interface VerifyReport {
  ok: boolean;
  entries: number;
  headHash: string;
  firstBroken: { id: number; reason: string } | null;
}

export interface AuditQuery {
  entityKind?: string;
  entityId?: number;
  beforeId?: number;
  limit?: number;
}

// ------------------------------------------------------------------ commands

export const api = {
  bootstrap: () => call<Bootstrap>('bootstrap'),
  status: () => call<AppStatus>('app_status'),
  setupShop: (shopName: string, techName: string, pin: string) =>
    call<SetupResult>('setup_shop', { shopName, techName, pin }),

  technicianNames: () => call<Technician[]>('technician_names'),
  unlock: (techId: number, pin: string) => call<TechSession>('unlock', { techId, pin }),
  lock: () => call<void>('lock'),
  touch: () => call<TechSession | null>('touch'),
  session: () => call<TechSession | null>('session'),
  recover: (code: string, techId: number, newPin: string) => call<string>('recover', { code, techId, newPin }),

  listTechnicians: () => call<Technician[]>('list_technicians'),
  addTechnician: (name: string, pin: string) => call<Technician>('add_technician', { name, pin }),
  updateTechnician: (id: number, name: string, active: boolean) =>
    call<Technician>('update_technician', { id, name, active }),
  setTechnicianPin: (id: number, pin: string) => call<void>('set_technician_pin', { id, pin }),
  regenerateRecoveryCode: () => call<string>('regenerate_recovery_code'),

  updateSettings: (settings: Settings) => call<Settings>('update_settings', { settings }),
  verifyAudit: () => call<VerifyReport>('verify_audit'),
  auditLog: (query: AuditQuery) => call<AuditRow[]>('audit_log', { query }),
};
