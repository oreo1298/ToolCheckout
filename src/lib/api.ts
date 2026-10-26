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
  loans: LoanRow[];
  incidents: Incident[];
}

export type CheckoutState = 'out' | 'awaiting_inspection' | 'closed' | 'lost';

export interface LoanRow {
  id: number;
  batchId: string;
  toolId: number;
  assetTag: string;
  toolName: string;
  toolPhotoId: number | null;
  toolStatus: ToolStatus;
  borrowerId: number;
  borrowerName: string;
  checkedOutAt: number;
  dueAt: number;
  checkoutCondition: Condition;
  authorizedBy: string | null;
  state: CheckoutState;
  returnedAt: number | null;
  dropoffNote: string;
  inspectedAt: number | null;
  inspectedBy: string | null;
  returnCondition: Condition | null;
  returnNotes: string;
  returnPhotoId: number | null;
  overdue: boolean;
  lateMs: number | null;
}

export interface AwaitingItem {
  kind: 'dropoff' | 'problem_report';
  toolId: number;
  assetTag: string;
  toolName: string;
  toolPhotoId: number | null;
  checkoutId: number | null;
  incidentId: number | null;
  borrowerId: number | null;
  borrowerName: string | null;
  since: number;
  note: string;
  conditionBefore: Condition;
  returnedLate: boolean;
}

export interface Home {
  overdue: LoanRow[];
  out: LoanRow[];
  awaiting: AwaitingItem[];
  openIncidents: number;
  availableTools: number;
  now: number;
}

export type ScanResult =
  | { kind: 'tool'; tool: ToolRow; loan: LoanRow | null }
  | { kind: 'borrower'; borrower: BorrowerPick }
  | { kind: 'unknown'; code: string; message: string };

export interface CheckoutItem {
  toolId: number;
  dueAt: number | null;
}

export interface CheckoutReceipt {
  batchId: string;
  borrower: Borrower;
  loans: LoanRow[];
}

export type IncidentKind = 'damage' | 'lost' | 'problem_report';
export type Outcome =
  | 'no_fault'
  | 'borrower_damaged'
  | 'repaired'
  | 'replaced'
  | 'borrower_paid'
  | 'found'
  | 'inspected'
  | 'written_off'
  | 'other';

export interface Incident {
  id: number;
  kind: IncidentKind;
  toolId: number;
  assetTag: string;
  toolName: string;
  replacementCostCents: number | null;
  borrowerId: number | null;
  borrowerName: string | null;
  checkoutId: number | null;
  beforeCondition: Condition | null;
  afterCondition: Condition | null;
  notes: string;
  photoIds: number[];
  createdAt: number;
  createdByKind: string;
  createdByName: string | null;
  status: 'open' | 'resolved';
  outcome: Outcome | null;
  resolutionNote: string | null;
  resolvedAt: number | null;
  resolvedByName: string | null;
}

export interface IncidentFilter {
  status?: 'open' | 'resolved';
  kind?: IncidentKind;
  borrowerId?: number;
  toolId?: number;
  from?: number;
  to?: number;
}

export interface InspectionResult {
  loan: LoanRow | null;
  tool: ToolRow;
  incident: Incident | null;
}

export interface LoanFilter {
  toolId?: number;
  borrowerId?: number;
  from?: number;
  to?: number;
  limit?: number;
}

export interface Borrower {
  id: number;
  name: string;
  employeeId: string;
  badgeCode: string | null;
  hasPin: boolean;
  active: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface BorrowerRow extends Borrower {
  openLoans: number;
  overdueLoans: number;
  lastCheckoutAt: number | null;
}

export interface BorrowerInput {
  name: string;
  employeeId: string;
  badgeCode: string | null;
}

export interface BorrowerPick {
  id: number;
  name: string;
  employeeId: string;
  hasPin: boolean;
  lastCheckoutAt: number | null;
  recentCheckouts: number;
}

export interface BorrowerAuth {
  token: string;
  borrower: Borrower;
}

export interface BorrowerDetail {
  row: BorrowerRow;
  loans: LoanRow[];
  incidents: Incident[];
}

export interface ReportFilter {
  from?: number;
  to?: number;
  borrowerId?: number;
  toolId?: number;
}

export interface BorrowerSummary {
  borrowerId: number;
  name: string;
  checkouts: number;
  returned: number;
  lateReturns: number;
  avgLateMs: number | null;
  currentlyOut: number;
  currentlyOverdue: number;
  damageIncidents: number;
  lostTools: number;
  problemsReported: number;
  costExposureCents: number;
}

export interface ToolSummary {
  toolId: number;
  assetTag: string;
  name: string;
  category: string;
  status: ToolStatus | null;
  condition: Condition | null;
  replacementCostCents: number | null;
  checkouts: number;
  lateReturns: number;
  timeOutMs: number;
  damageIncidents: number;
  lostIncidents: number;
  problemReports: number;
}

export interface BackupFile {
  path: string;
  name: string;
  size: number;
  modified: number;
  automatic: boolean;
}

export interface BackupStatus {
  dir: string;
  lastAutoBackup: number | null;
  files: BackupFile[];
}

export interface BackupInfo {
  schemaVersion: number;
  tools: number;
  borrowers: number;
  checkouts: number;
  lastActivity: number | null;
  shopName: string;
  auditOk: boolean;
  auditEntries: number;
  demo: boolean;
}

export interface BackupCheck {
  name: string;
  problem: string | null;
}

export interface FolderInfo {
  path: string;
  existing: BackupInfo | null;
  writable: boolean;
}

export type ReportKind = 'borrowers' | 'tools' | 'loans' | 'incidents' | 'open_incidents';

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

  borrowerPicker: () => call<BorrowerPick[]>('borrower_picker'),
  listBorrowers: (filter: { search?: string; includeInactive?: boolean } = {}) =>
    call<BorrowerRow[]>('list_borrowers', { filter }),
  getBorrower: (id: number) => call<Borrower>('get_borrower', { id }),
  createBorrower: (input: BorrowerInput, pin: string | null) => call<Borrower>('create_borrower', { input, pin }),
  updateBorrower: (id: number, input: BorrowerInput, active: boolean) =>
    call<Borrower>('update_borrower', { id, input, active }),
  setBorrowerPin: (id: number, pin: string) => call<void>('set_borrower_pin', { id, pin }),
  authenticateBorrower: (id: number, pin: string | null) => call<BorrowerAuth>('authenticate_borrower', { id, pin }),

  borrowerDetail: (id: number) => call<BorrowerDetail>('borrower_detail', { id }),

  home: () => call<Home>('home'),
  resolveScan: (code: string) => call<ScanResult>('resolve_scan', { code }),
  openLoanForTool: (toolId: number) => call<LoanRow | null>('open_loan_for_tool', { toolId }),
  checkout: (token: string, items: CheckoutItem[]) => call<CheckoutReceipt>('checkout', { token, items }),
  reportProblem: (token: string, toolId: number, notes: string, photoId: number | null) =>
    call<Incident>('report_problem', { token, toolId, notes, photoId }),
  dropOff: (token: string, checkoutIds: number[], note: string) =>
    call<LoanRow[]>('drop_off', { token, checkoutIds, note }),
  inspectReturn: (checkoutId: number, condition: Condition, notes: string, photoId: number | null) =>
    call<InspectionResult>('inspect_return', { checkoutId, condition, notes, photoId }),
  inspectTool: (toolId: number, condition: Condition, notes: string, photoId: number | null) =>
    call<InspectionResult>('inspect_tool', { toolId, condition, notes, photoId }),
  markLost: (toolId: number, note: string) => call<Incident>('mark_lost', { toolId, note }),
  markFound: (toolId: number, condition: Condition, notes: string, photoId: number | null) =>
    call<InspectionResult>('mark_found', { toolId, condition, notes, photoId }),
  loanHistory: (filter: LoanFilter) => call<LoanRow[]>('loan_history', { filter }),
  listIncidents: (filter: IncidentFilter) => call<Incident[]>('list_incidents', { filter }),
  resolveIncident: (id: number, outcome: Outcome, note: string) =>
    call<Incident>('resolve_incident', { id, outcome, note }),

  reportBorrowers: (filter: ReportFilter) => call<BorrowerSummary[]>('report_borrowers', { filter }),
  reportTools: (filter: ReportFilter) => call<ToolSummary[]>('report_tools', { filter }),
  reportLoans: (filter: ReportFilter) => call<LoanRow[]>('report_loans', { filter }),
  reportIncidents: (filter: ReportFilter, openOnly: boolean) =>
    call<Incident[]>('report_incidents', { filter, openOnly }),
  exportReportCsv: (kind: ReportKind, filter: ReportFilter, fileName: string) =>
    call<string | null>('export_report_csv', { kind, filter, fileName }),

  backupStatus: () => call<BackupStatus>('backup_status'),
  backupNow: () => call<string>('backup_now'),
  saveBackupAs: () => call<string | null>('save_backup_as'),
  pickBackupFile: () => call<{ path: string; info: BackupInfo } | null>('pick_backup_file'),
  restoreBackup: (path: string) => call<string>('restore_backup', { path }),
  verifyAgainstBackups: () => call<BackupCheck[]>('verify_against_backups'),
  pickFolder: (title: string) => call<string | null>('pick_folder', { title }),
  inspectDataFolder: (path: string) => call<FolderInfo>('inspect_data_folder', { path }),
  relocateData: (path: string | null, copy: boolean) => call<DataLocation>('relocate_data', { path, copy }),
  retryOpen: () => call<void>('retry_open'),
  enterDemo: () => call<void>('enter_demo'),
  exitDemo: () => call<void>('exit_demo'),

  savePhoto: (data: string) => call<number>('save_photo', { data }),
  getPhoto: (id: number) => call<string>('get_photo', { id }),
};
