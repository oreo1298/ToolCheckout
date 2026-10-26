<script lang="ts">
  import { api } from './lib/api';
  import { app, type Screen } from './lib/app.svelte';
  import Icon from './lib/components/Icon.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import { countdown } from './lib/format';
  import UnlockDialog from './lib/flows/UnlockDialog.svelte';
  import CheckoutSheet from './lib/flows/CheckoutSheet.svelte';
  import InspectSheet from './lib/flows/InspectSheet.svelte';
  import ReturnSheet from './lib/flows/ReturnSheet.svelte';
  import Borrowers from './lib/screens/Borrowers.svelte';
  import Home from './lib/screens/Home.svelte';
  import Incidents from './lib/screens/Incidents.svelte';
  import Reports from './lib/screens/Reports.svelte';
  import Settings from './lib/screens/Settings.svelte';
  import Setup from './lib/screens/Setup.svelte';
  import Tools from './lib/screens/Tools.svelte';
  import ToolDetail from './lib/tools/ToolDetail.svelte';

  const nav: { id: Screen; label: string; icon: string; tech?: boolean }[] = [
    { id: 'home', label: 'Home', icon: 'home' },
    { id: 'tools', label: 'Tools', icon: 'tool', tech: true },
    { id: 'borrowers', label: 'Borrowers', icon: 'users', tech: true },
    { id: 'incidents', label: 'Incidents', icon: 'alert', tech: true },
    { id: 'reports', label: 'Reports', icon: 'chart', tech: true },
    { id: 'settings', label: 'Settings', icon: 'settings', tech: true },
  ];

  async function go(item: (typeof nav)[number]) {
    if (item.tech && !app.session && !(await app.requestUnlock(`${item.label} needs a technician PIN.`))) return;
    app.screen = item.id;
  }

  // Recovering from "can't open its data" (e.g. a shared folder's drive was
  // missing at startup).
  async function retry() {
    try {
      await api.retryOpen();
    } catch {
      // The screen shows the new reason.
    }
    await app.reload();
  }

  async function useDefault() {
    try {
      await api.relocateData(null, false);
    } catch {
      // As above.
    }
    await app.reload();
  }

  const remaining = $derived(app.session ? app.session.expiresAt - app.nowMs : 0);
</script>

{#if !app.boot}
  <div class="center muted">Starting…</div>
{:else if app.boot.startupError}
  <div class="center">
    <div class="card card-pad fatal stack">
      <h1>ToolCheckout can't open its data</h1>
      <p>{app.boot.startupError}</p>
      <p class="muted">
        Check that the folder exists and you can write to it. If the data is in a shared folder on another drive, make
        sure the drive is connected, then try again.
      </p>
      <div class="row wrap">
        <button class="btn btn-primary" onclick={retry}>Try again</button>
        {#if app.boot.location?.mode === 'custom'}
          <button class="btn" onclick={useDefault}>Use this account's own data folder instead</button>
        {/if}
      </div>
    </div>
  </div>
{:else if app.onboarding || !app.status?.setUp}
  <Setup />
{:else}
  <div class="shell">
    <header class="topbar">
      <div class="brand">
        <span class="logo" aria-hidden="true"><Icon name="tool" size={18} /></span>
        <span class="shop">{app.settings?.shopName}</span>
      </div>
      {#if app.status.demo}
        <span class="demo" title="Sample data. Wipe or leave demo mode in Settings.">DEMO DATA</span>
      {/if}
      <span class="spacer"></span>
      {#if app.session}
        <div class="unlocked" role="status">
          <Icon name="unlock" size={18} />
          <span>Unlocked: <strong>{app.session.techName}</strong></span>
          <span class="timer" title="Locks after inactivity">{countdown(remaining)}</span>
        </div>
        <button class="btn btn-sm lockbtn" onclick={() => app.lock()} title="Lock tech mode now">
          <Icon name="lock" size={18} /> Lock
        </button>
      {:else}
        <button class="btn btn-sm" onclick={() => app.requestUnlock()}>
          <Icon name="lock" size={18} /> Tech mode
        </button>
      {/if}
    </header>

    <nav class="rail" aria-label="Main">
      {#each nav as item (item.id)}
        <button
          class="rail-item"
          class:active={app.screen === item.id}
          aria-current={app.screen === item.id ? 'page' : undefined}
          onclick={() => go(item)}
        >
          <span class="ico">
            <Icon name={item.icon} size={24} />
            {#if item.tech && !app.session}<span class="mini-lock"><Icon name="lock" size={11} stroke={2.5} /></span>{/if}
          </span>
          <span>{item.label}</span>
        </button>
      {/each}
    </nav>

    <main class="content">
      {#if app.screen === 'home'}
        <Home />
      {:else if app.screen === 'settings'}
        <Settings />
      {:else if app.screen === 'incidents'}
        <Incidents />
      {:else if app.screen === 'reports'}
        <Reports />
      {:else if app.screen === 'tools'}
        <Tools />
      {:else if app.screen === 'borrowers'}
        <Borrowers />
      {:else}
        <h1>{nav.find((n) => n.id === app.screen)?.label}</h1>
        <p class="muted">Coming up in the next milestones.</p>
      {/if}
    </main>
  </div>
{/if}

{#if app.flow?.kind === 'checkout'}
  <CheckoutSheet tools={app.flow.tools} borrower={app.flow.borrower} onclose={() => (app.flow = null)} />
{:else if app.flow?.kind === 'return'}
  <ReturnSheet loans={app.flow.loans} onclose={() => (app.flow = null)} />
{:else if app.flow?.kind === 'inspect'}
  <InspectSheet item={app.flow.item} onclose={() => (app.flow = null)} />
{/if}
{#if app.toolDetailId !== null && app.session}
  {#key app.toolDetailId}
    <ToolDetail id={app.toolDetailId} onclose={() => (app.toolDetailId = null)} />
  {/key}
{/if}
{#if app.unlockRequest}
  <UnlockDialog reason={app.unlockRequest.reason} />
{/if}
<Toasts />

<style>
  .center {
    min-height: 100%;
    display: grid;
    place-items: center;
    padding: 2rem;
  }
  .fatal {
    max-width: 640px;
  }
  .shell {
    display: grid;
    grid-template-columns: 96px 1fr;
    grid-template-rows: 60px 1fr;
    height: 100%;
  }
  .topbar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0 1rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-weight: 700;
    font-size: 1.05rem;
    min-width: 0;
  }
  .shop {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .logo {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-contrast);
    flex: none;
  }
  .demo {
    padding: 0.2rem 0.6rem;
    border-radius: 6px;
    background: var(--warning-soft);
    color: var(--warning);
    font-weight: 800;
    font-size: 0.8rem;
    letter-spacing: 0.08em;
    border: 1px dashed var(--warning);
  }
  .unlocked {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    background: var(--success-soft);
    color: var(--success);
  }
  .unlocked span {
    color: var(--text);
  }
  .timer {
    font-variant-numeric: tabular-nums;
    font-size: 0.85rem;
    color: var(--text-muted) !important;
  }
  .lockbtn {
    border-color: var(--success);
  }
  .rail {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.5rem;
    background: var(--surface);
    border-right: 1px solid var(--border);
    overflow-y: auto;
  }
  .rail-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    min-height: 68px;
    justify-content: center;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
  }
  .rail-item:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .rail-item.active {
    background: var(--accent-soft);
    color: var(--text);
  }
  .rail-item.active .ico {
    color: var(--accent);
  }
  .ico {
    position: relative;
  }
  .mini-lock {
    position: absolute;
    right: -7px;
    bottom: -3px;
    background: var(--surface);
    border-radius: 50%;
    padding: 1px;
    color: var(--text-faint);
  }
  .content {
    overflow: auto;
    padding: 1.25rem 1.5rem 2rem;
  }
</style>
