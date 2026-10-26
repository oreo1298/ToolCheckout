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

export type Condition = 'new' | 'good' | 'fair' | 'worn' | 'damaged' | 'broken';
export type ToolStatus = 'available' | 'checked_out' | 'awaiting_inspection' | 'out_for_repair' | 'lost' | 'retired';
export type ConditionSource = 'initial' | 'manual' | 'return_inspection' | 'inspection' | 'repair' | 'found';

export interface Tool {
  id: number;
  assetTag: string;
  name: string;
  category: string;
  description: string;
  serial: string;
  model: string;
  photoId: number | null;
  replacementCostCents: number | null;
  homeLocation: string;
  defaultLoanMinutes: number;
  techOnly: boolean;
  condition: Condition;
  conditionNotes: string;
  status: ToolStatus;
  createdAt: number;
  updatedAt: number;
}

export interface ToolRow extends Tool {
  checkoutId: number | null;
  borrowerId: number | null;
  borrowerName: string | null;
  checkedOutAt: number | null;
  dueAt: number | null;
}

export interface ToolInput {
  name: string;
  category: string;
  description: string;
  serial: string;
  model: string;
  photoId: number | null;
  replacementCostCents: number | null;
  homeLocation: string;
  defaultLoanMinutes: number;
  techOnly: boolean;
  condition: Condition | null;
  conditionNotes: string;
}

export interface ToolFilter {
  search?: string;
  status?: ToolStatus;
  category?: string;
  includeRetired?: boolean;
  limit?: number;
  id?: number;
}

export interface ConditionEvent {
  id: number;
  toolId: number;
  condition: Condition;
  notes: string;
  photoId: number | null;
  reason: string;
  source: ConditionSource;
  checkoutId: number | null;
  techName: string | null;
  createdAt: number;
}

export interface ToolDetail {
  row: ToolRow;
  conditions: ConditionEvent[];
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

  listTools: (filter: ToolFilter = {}) => call<ToolRow[]>('list_tools', { filter }),
  toolRow: (id: number) => call<ToolRow>('tool_row', { id }),
  toolCategories: () => call<string[]>('tool_categories'),
  toolDetail: (id: number) => call<ToolDetail>('tool_detail', { id }),
  createTool: (input: ToolInput) => call<Tool>('create_tool', { input }),
  updateTool: (id: number, input: ToolInput) => call<Tool>('update_tool', { id, input }),
  updateToolCondition: (id: number, condition: Condition, notes: string, photoId: number | null, reason: string) =>
    call<Tool>('update_tool_condition', { id, condition, notes, photoId, reason }),
  sendToolToRepair: (id: number, note: string) => call<Tool>('send_tool_to_repair', { id, note }),
  toolBackFromRepair: (id: number, condition: Condition, notes: string, photoId: number | null) =>
    call<Tool>('tool_back_from_repair', { id, condition, notes, photoId }),
  retireTool: (id: number, reason: string) => call<Tool>('retire_tool', { id, reason }),
  reinstateTool: (id: number, condition: Condition, notes: string) =>
    call<Tool>('reinstate_tool', { id, condition, notes }),

  savePhoto: (data: string) => call<number>('save_photo', { data }),
  getPhoto: (id: number) => call<string>('get_photo', { id }),
};
