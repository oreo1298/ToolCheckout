// Smoke test: the real app shell against a mocked Tauri backend.
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { render, screen, waitFor, within } from '@testing-library/svelte';
import App from './App.svelte';
import type { AppStatus, Home, LoanRow, Settings, ToolRow } from './lib/api';
import { app } from './lib/app.svelte';

const NOW = new Date(2026, 4, 4, 14, 0).getTime();
const settings: Settings = {
  shopName: 'Smoke Test Repair',
  defaultLoanMinutes: 240,
  borrowerPinRequired: true,
  techPinForCheckout: false,
  autoLockSeconds: 120,
  theme: 'system',
  overdueNotifications: true,
  overdueReminderMinutes: 60,
  endOfDayReminder: false,
  closingTime: '17:30',
  autoBackup: true,
  backupDir: null,
  backupKeep: 30,
  currencySymbol: '$',
  labelStock: 'avery5160',
};

const tool = (over: Partial<ToolRow> = {}): ToolRow => ({
  id: 1,
  assetTag: 'T-0001',
  name: 'Pentalobe P5 driver',
  category: 'Drivers',
  description: '',
  serial: '',
  model: '',
  photoId: null,
  replacementCostCents: 899,
  homeLocation: 'Bench 1',
  defaultLoanMinutes: 240,
  techOnly: false,
  condition: 'good',
  conditionNotes: '',
  status: 'available',
  createdAt: NOW,
  updatedAt: NOW,
  checkoutId: null,
  borrowerId: null,
  borrowerName: null,
  checkedOutAt: null,
  dueAt: null,
  ...over,
});

const loan = (over: Partial<LoanRow>): LoanRow => ({
  id: 1,
  batchId: 'b',
  toolId: 2,
  assetTag: 'T-0002',
  toolName: 'Fluke 117 multimeter',
  toolPhotoId: null,
  toolStatus: 'checked_out',
  borrowerId: 7,
  borrowerName: 'Marcus Brown',
  checkedOutAt: NOW - 5 * 3_600_000,
  dueAt: NOW - 2 * 3_600_000,
  checkoutCondition: 'good',
  authorizedBy: null,
  state: 'out',
  returnedAt: null,
  dropoffNote: '',
  inspectedAt: null,
  inspectedBy: null,
  returnCondition: null,
  returnNotes: '',
  returnPhotoId: null,
  overdue: true,
  lateMs: 2 * 3_600_000,
  ...over,
});

function backend(setUp: boolean, opts: { startupError?: string } = {}) {
  const status: AppStatus = { setUp, demo: false, settings, session: null, schemaVersion: 1, dbPath: '/tmp/x.db', now: NOW };
  let startupError = opts.startupError ?? null;
  let mode = startupError ? 'custom' : 'default';
  const home: Home = {
    overdue: [loan({})],
    out: [loan({ id: 2, toolId: 3, assetTag: 'T-0003', toolName: 'Heat gun', borrowerName: 'Priya Shah', overdue: false, lateMs: null, dueAt: NOW + 3_600_000 })],
    awaiting: [],
    openIncidents: 1,
    availableTools: 9,
    now: NOW,
  };
  const calls: string[] = [];
  const relocations: unknown[] = [];
  mockIPC(
    (cmd, args) => {
      calls.push(cmd);
      switch (cmd) {
        case 'bootstrap':
          return {
            status: startupError ? null : status,
            location: { dir: '/tmp', mode, defaultDir: '/tmp' },
            startupError,
            platform: 'linux',
            version: '0.1.0',
          };
        case 'pick_folder':
          return '/shared/ToolCheckout';
        case 'inspect_data_folder':
          return {
            path: (args as { path: string }).path,
            writable: true,
            existing: { schemaVersion: 1, tools: 12, borrowers: 1, checkouts: 40, lastActivity: NOW, shopName: 'Shared Shop', auditOk: true, auditEntries: 90, demo: false },
          };
        case 'relocate_data':
          relocations.push(args);
          status.setUp = true;
          startupError = null;
          mode = 'custom';
          return { dir: '/shared/ToolCheckout', mode, defaultDir: '/tmp' };
        case 'app_status':
          return status;
        case 'home':
          return home;
        case 'borrower_picker':
          return [{ id: 7, name: 'Marcus Brown', employeeId: 'E1003', hasPin: true, lastCheckoutAt: NOW, recentCheckouts: 3 }];
        case 'list_tools':
          return [];
        case 'resolve_scan':
          return (args as { code: string }).code.toUpperCase() === 'T-0001'
            ? { kind: 'tool', tool: tool(), loan: null }
            : { kind: 'unknown', code: 'x', message: 'Nothing matches' };
        case 'technician_names':
          return [{ id: 1, name: 'Ana Ruiz', active: true, createdAt: NOW }];
        case 'get_os_appearance':
          return { colorScheme: null, accent: null, source: 'test' };
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
  return Object.assign(calls, { relocations });
}

function scan(code: string) {
  for (const key of [...code, 'Enter']) {
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  }
}

afterEach(() => {
  clearMocks();
  app.flow = null;
  app.unlockRequest = null;
});

describe('App shell', () => {
  it('shows the home screen with overdue and current loans', async () => {
    backend(true);
    await app.init();
    render(App);
    expect(await screen.findByText('Smoke Test Repair')).toBeInTheDocument();
    const overdue = (await screen.findByRole('heading', { name: /Overdue/ })).closest('section')!;
    expect(within(overdue).getByText('Marcus Brown')).toBeInTheDocument();
    expect(within(overdue).getByText(/2h overdue/)).toBeInTheDocument();
    expect(screen.getByText('Heat gun')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Check out/ })).toBeInTheDocument();
  });

  it('scanning an available tool tag starts a checkout with that tool', async () => {
    const calls = backend(true);
    await app.init();
    render(App);
    await screen.findByText('Smoke Test Repair');
    scan('T-0001');
    const dialog = await screen.findByRole('dialog', { name: 'Check out' });
    expect(within(dialog).getByText('Pentalobe P5 driver')).toBeInTheDocument();
    expect(await within(dialog).findByText('Marcus Brown')).toBeInTheDocument();
    expect(calls).toContain('resolve_scan');
  });

  it('technician-only sections ask for a PIN first', async () => {
    backend(true);
    await app.init();
    render(App);
    (await screen.findByRole('button', { name: /Settings/ })).click();
    const dialog = await screen.findByRole('dialog', { name: 'Technician PIN' });
    await waitFor(() => expect(within(dialog).getByText('Ana Ruiz')).toBeInTheDocument());
    expect(within(dialog).getByText(/Settings needs a technician PIN/)).toBeInTheDocument();
  });

  it('first run shows setup instead of the app', async () => {
    backend(false);
    await app.init();
    render(App);
    expect(await screen.findByLabelText('Shop name')).toBeInTheDocument();
    expect(screen.getByText(/Try the demo/)).toBeInTheDocument();
  });

  it('first run can switch to a folder another account already shares', async () => {
    const { relocations } = backend(false);
    await app.init();
    render(App);
    (await screen.findByRole('button', { name: /Use its shared folder/ })).click();
    const dialog = await screen.findByRole('dialog', { name: 'Use this shared data?' });
    expect(within(dialog).getByText('Shared Shop')).toBeInTheDocument();
    expect(within(dialog).getByText(/12 tools, 1 borrower\)/)).toBeInTheDocument();
    within(dialog).getByRole('button', { name: 'Use this data' }).click();
    expect(await screen.findByRole('button', { name: /Check out/ })).toBeInTheDocument();
    expect(relocations).toEqual([{ path: '/shared/ToolCheckout', copy: false }]);
  });

  it("offers a way out when the data folder can't be opened", async () => {
    const { relocations } = backend(true, { startupError: 'Could not open the ToolCheckout data in Z:\\Shop' });
    await app.init();
    render(App);
    expect(await screen.findByText(/Could not open the ToolCheckout data/)).toBeInTheDocument();
    (await screen.findByRole('button', { name: /own data folder/ })).click();
    expect(await screen.findByRole('button', { name: /Check out/ })).toBeInTheDocument();
    expect(relocations).toEqual([{ path: null, copy: false }]);
  });
});
