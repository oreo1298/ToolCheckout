<script lang="ts">
  // Data folder, backups (automatic, manual, restore) and demo mode.
  import { api, type BackupInfo, type BackupStatus, type FolderInfo, type Settings } from '../api';
  import { app } from '../app.svelte';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import Icon from '../components/Icon.svelte';
  import Modal from '../components/Modal.svelte';
  import { count, dateTime } from '../format';

  let status = $state<BackupStatus | null>(null);
  let restoring = $state<{ path: string; info: BackupInfo } | null>(null);
  let moving = $state<FolderInfo | null>(null);
  let busy = $state(false);
  let askDemo = $state(false);

  const loc = $derived(app.boot?.location ?? null);
  const fixedLocation = $derived(loc?.mode === 'portable' || loc?.mode === 'environment');
  const demo = $derived(app.status?.demo ?? false);

  $effect(() => {
    void app.dataVersion;
    if (app.session) api.backupStatus().then((s) => (status = s), () => {});
  });

  async function saveSettings(patch: Partial<Settings>) {
    try {
      const next = { ...$state.snapshot(app.settings!), ...patch } as Settings;
      if (await app.withTech(() => api.updateSettings(next))) {
        await app.refreshStatus();
        app.toast('Backup settings saved');
      }
    } catch (e) {
      app.error(e);
    }
  }

  async function chooseBackupFolder() {
    const dir = await api.pickFolder('Choose a folder for automatic backups');
    if (dir) await saveSettings({ backupDir: dir });
  }

  async function run<T>(fn: () => Promise<T>, ok: (v: T) => string) {
    busy = true;
    try {
      const v = await app.withTech(fn);
      if (v !== undefined && v !== null) app.toast(ok(v));
    } catch (e) {
      app.error(e);
    } finally {
      busy = false;
    }
  }

  async function pickRestore() {
    try {
      const r = await app.withTech(() => api.pickBackupFile());
      if (r) restoring = r;
    } catch (e) {
      app.error(e);
    }
  }

  async function confirmRestore() {
    if (!restoring) return;
    busy = true;
    try {
      const safety = await app.withTech(() => api.restoreBackup(restoring!.path));
      if (safety) {
        restoring = null;
        await app.reload();
        app.toast(`Backup restored. Your previous data was saved to ${safety}`);
      }
    } catch (e) {
      app.error(e);
    } finally {
      busy = false;
    }
  }

  async function pickDataFolder() {
    try {
      const dir = await api.pickFolder('Choose the shared data folder');
      if (!dir) return;
      const info = await app.withTech(() => api.inspectDataFolder(dir));
      if (info) moving = info;
    } catch (e) {
      app.error(e);
    }
  }

  async function relocate(path: string | null, copy: boolean) {
    busy = true;
    try {
      const l = await app.withTech(() => api.relocateData(path, copy));
      if (l) {
        moving = null;
        await app.reload();
        app.toast(`ToolCheckout now uses ${l.dir}`);
      }
    } catch (e) {
      app.error(e);
    } finally {
      busy = false;
    }
  }

  async function enterDemo() {
    askDemo = false;
    await run(async () => (await api.enterDemo(), true), () => 'Demo data loaded');
    await app.reload();
  }

  async function exitDemo() {
    try {
      await api.exitDemo();
      await app.reload();
      app.toast('Left the demo. Demo data was wiped.');
    } catch (e) {
      app.error(e);
    }
  }

  async function resetDemo() {
    try {
      await api.enterDemo();
      await app.reload();
      app.toast('Demo data reset');
    } catch (e) {
      app.error(e);
    }
  }

  const MODES: Record<string, string> = {
    default: 'Default (this Windows/Linux user)',
    custom: 'Custom folder',
    portable: 'Portable (next to the program)',
    environment: 'Set by TOOLCHECKOUT_DATA_DIR',
  };
  const kb = (n: number) => (n > 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1024))} KB`);
</script>

<section class="card card-pad stack">
  <h2>Data &amp; backups</h2>

  {#if demo}
    <div class="demo">
      <Icon name="box" />
      <div class="grow">
        <strong>You're using demo data.</strong> Your real shop data is untouched.
      </div>
      <button class="btn btn-sm" onclick={resetDemo}>Reset demo data</button>
      <button class="btn btn-sm btn-primary" onclick={exitDemo}>Leave demo (wipe it)</button>
    </div>
  {/if}

  <div class="field">
    <span class="label">Data folder</span>
    <div class="row wrap">
      <code class="path grow">{loc?.dir}</code>
      <span class="badge badge-neutral">{MODES[loc?.mode ?? 'default']}</span>
    </div>
    {#if !fixedLocation && !demo}
      <div class="row wrap">
        <button class="btn btn-sm" onclick={pickDataFolder}><Icon name="database" size={16} /> Use a different folder…</button>
        {#if loc?.mode === 'custom'}
          <button class="btn btn-sm btn-ghost" disabled={busy} onclick={() => relocate(null, false)}>Back to default folder</button>
        {/if}
      </div>
      <p class="faint small">
        To share data between Windows accounts on this PC, choose a folder all of them can write to, such as
        <code>C:\Users\Public\ToolCheckout</code>, then pick the same folder in each account.
      </p>
    {/if}
  </div>

  <h3>Backups</h3>
  {#if demo}
    <p class="muted">Backups and restore are off while you're using demo data.</p>
  {:else}
    <label class="check">
      <input type="checkbox" checked={app.settings?.autoBackup} onchange={(e) => saveSettings({ autoBackup: (e.currentTarget as HTMLInputElement).checked })} />
      Back up automatically once a day, keeping the last
      <input
        class="input keep"
        type="number"
        min="1"
        max="365"
        value={app.settings?.backupKeep}
        onchange={(e) => saveSettings({ backupKeep: Number((e.currentTarget as HTMLInputElement).value) })}
        aria-label="Number of automatic backups to keep"
      />
    </label>
    <div class="row wrap">
      <span class="label">Folder:</span>
      <code class="path grow">{status?.dir ?? '…'}</code>
      <button class="btn btn-sm" onclick={chooseBackupFolder}>Change…</button>
      {#if app.settings?.backupDir}
        <button class="btn btn-sm btn-ghost" onclick={() => saveSettings({ backupDir: null })}>Use default</button>
      {/if}
    </div>
    <p class="faint small">
      Tip: point backups at a different drive or a synced folder. A backup on the same disk won't survive a dead disk, and
      copies elsewhere also make tampering with the history easier to spot.
      {#if status?.lastAutoBackup}Last automatic backup: {dateTime(status.lastAutoBackup)}.{/if}
    </p>
    <div class="row wrap">
      <button class="btn" disabled={busy} onclick={() => run(() => api.backupNow(), (p) => `Backup saved: ${p}`)}><Icon name="database" /> Back up now</button>
      <button class="btn" disabled={busy} onclick={() => run(() => api.saveBackupAs(), (p) => `Backup saved: ${p}`)}><Icon name="download" /> Save a copy to…</button>
      <span class="spacer"></span>
      <button class="btn" disabled={busy} onclick={pickRestore}><Icon name="upload" /> Restore from backup…</button>
    </div>
    {#if status?.files.length}
      <details>
        <summary>{count(status.files.length, 'backup')} in the folder</summary>
        <ul class="files">
          {#each status.files.slice(0, 40) as f (f.path)}
            <li><span class="mono grow">{f.name}</span><span class="faint">{kb(f.size)}</span><span class="faint">{dateTime(f.modified)}</span></li>
          {/each}
        </ul>
      </details>
    {/if}
  {/if}

  {#if !demo}
    <h3>Demo</h3>
    <div class="row wrap">
      <p class="muted grow">Explore ToolCheckout with sample tools, people and history. Your shop data stays untouched.</p>
      <button class="btn" onclick={() => (askDemo = true)}><Icon name="box" /> Open demo</button>
    </div>
  {/if}
</section>

{#if askDemo}
  <ConfirmDialog title="Open the demo?" confirmLabel="Open demo" onconfirm={enterDemo} onclose={() => (askDemo = false)}>
    <p>ToolCheckout switches to sample data. Your shop data stays untouched, and you can come back any time.</p>
  </ConfirmDialog>
{/if}

{#if restoring}
  <Modal title="Restore this backup?" size="md" onclose={() => (restoring = null)}>
    <div class="stack">
      <code class="path">{restoring.path}</code>
      <dl>
        <dt>Shop</dt><dd>{restoring.info.shopName}</dd>
        <dt>Last activity</dt><dd>{restoring.info.lastActivity ? dateTime(restoring.info.lastActivity) : '—'}</dd>
        <dt>Contents</dt><dd>{count(restoring.info.tools, 'tool')} · {count(restoring.info.borrowers, 'borrower')} · {count(restoring.info.checkouts, 'loan')}</dd>
        <dt>Audit log</dt>
        <dd>
          {#if restoring.info.auditOk}<span class="badge badge-success">Verified</span> {restoring.info.auditEntries} entries{:else}<span class="badge badge-danger">Fails verification</span>{/if}
        </dd>
      </dl>
      <p>
        <strong>Everything currently in ToolCheckout will be replaced</strong> by this backup. A safety copy of the current data
        is saved to the backup folder first, and the restore is recorded in the audit log.
      </p>
    </div>
    {#snippet footer()}
      <button class="btn" onclick={() => (restoring = null)}>Cancel</button>
      <button class="btn btn-danger" disabled={busy || !restoring?.info.auditOk} onclick={confirmRestore}>Replace data with this backup</button>
    {/snippet}
  </Modal>
{/if}

{#if moving}
  <Modal title="Use this data folder?" size="md" onclose={() => (moving = null)}>
    <div class="stack">
      <code class="path">{moving.path}</code>
      {#if !moving.writable}
        <p class="error-text">ToolCheckout can't write to this folder. Choose another one or fix its permissions.</p>
      {:else if moving.existing}
        <p>
          This folder already has ToolCheckout data for <strong>{moving.existing.shopName}</strong>
          ({count(moving.existing.tools, 'tool')}, {count(moving.existing.borrowers, 'borrower')}). Switch to it? The data here now is left
          where it is.
        </p>
      {:else}
        <p>
          ToolCheckout will copy its data there and use that folder from now on. The current files are left in place as a
          fallback. Other Windows accounts can then point at the same folder.
        </p>
      {/if}
    </div>
    {#snippet footer()}
      <button class="btn" onclick={() => (moving = null)}>Cancel</button>
      {#if moving?.writable}
        {#if moving.existing}
          <button class="btn btn-primary" disabled={busy} onclick={() => relocate(moving!.path, false)}>Switch to that data</button>
        {:else}
          <button class="btn btn-primary" disabled={busy} onclick={() => relocate(moving!.path, true)}>Copy data and switch</button>
        {/if}
      {/if}
    {/snippet}
  </Modal>
{/if}

<style>
  .path {
    font-family: var(--mono);
    font-size: 0.88rem;
    padding: 0.35rem 0.6rem;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    word-break: break-all;
    white-space: normal;
  }
  .small {
    font-size: 0.85rem;
  }
  .keep {
    width: 80px;
    min-height: 38px;
    padding: 0.2rem 0.5rem;
    display: inline-block;
  }
  .files {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0 0;
    max-height: 260px;
    overflow: auto;
  }
  .files li {
    display: flex;
    gap: 1rem;
    padding: 0.3rem 0;
    border-bottom: 1px solid var(--border);
    font-size: 0.88rem;
  }
  .demo {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.8rem 1rem;
    border-radius: var(--radius);
    background: var(--warning-soft);
    border: 1px dashed var(--warning);
    flex-wrap: wrap;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.25rem 1rem;
    margin: 0;
  }
  dt {
    color: var(--text-muted);
  }
  dd {
    margin: 0;
  }
</style>
