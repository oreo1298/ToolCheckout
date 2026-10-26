<script lang="ts">
  // First run: shop name + first technician + PIN, then the one-time
  // recovery code.
  import { api, type FolderInfo } from '../api';
  import { app } from '../app.svelte';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import Icon from '../components/Icon.svelte';
  import PinCreate from '../components/PinCreate.svelte';
  import RecoveryCode from '../components/RecoveryCode.svelte';
  import { count } from '../format';

  let demoBusy = $state(false);
  async function ondemo() {
    demoBusy = true;
    try {
      await api.enterDemo();
      await app.reload();
      app.onboarding = false;
      app.toast('Demo data loaded — have a look around!');
    } catch (e) {
      app.error(e);
    } finally {
      demoBusy = false;
    }
  }

  // Another account on this PC already keeps its data in a shared folder.
  const canShare = $derived(!['portable', 'environment'].includes(app.boot?.location?.mode ?? ''));
  let shared = $state<FolderInfo | null>(null);
  async function pickShared() {
    try {
      const dir = await api.pickFolder('Choose the shared ToolCheckout data folder');
      if (!dir) return;
      const info = await api.inspectDataFolder(dir);
      if (!info.existing) {
        app.toast("There's no ToolCheckout data in that folder. Choose the folder the other account uses.", 'error');
      } else if (!info.writable) {
        app.toast("This account can't write to that folder. Check the folder's permissions.", 'error');
      } else {
        shared = info;
      }
    } catch (e) {
      app.error(e);
    }
  }
  async function useShared() {
    const path = shared!.path;
    shared = null;
    try {
      await api.relocateData(path, false);
      await app.reload();
      app.onboarding = false;
      app.toast(`Using the shared data in ${path}`);
    } catch (e) {
      app.error(e);
    }
  }

  let step = $state<'names' | 'pin' | 'code'>('names');
  let shopName = $state('');
  let techName = $state('');
  let error = $state<string | null>(null);
  let recoveryCode = $state('');

  function next(e: SubmitEvent) {
    e.preventDefault();
    error = null;
    if (!shopName.trim() || !techName.trim()) {
      error = 'Please fill in both names.';
      return;
    }
    step = 'pin';
  }

  async function create(pin: string) {
    const r = await api.setupShop(shopName, techName, pin);
    recoveryCode = r.recoveryCode;
    step = 'code';
  }

  async function finish() {
    await app.refreshStatus();
    app.onboarding = false;
    app.toast(`Welcome to ToolCheckout, ${techName.trim()}!`);
  }
</script>

<div class="setup">
  <div class="card panel">
    <div class="brand">
      <span class="logo"><Icon name="tool" size={26} /></span>
      <div>
        <h1>ToolCheckout</h1>
        <p class="muted">Fast, accountable tool loans for the shop.</p>
      </div>
    </div>

    <ol class="steps" aria-label="Setup steps">
      <li class:on={step === 'names'}>Shop</li>
      <li class:on={step === 'pin'}>Your PIN</li>
      <li class:on={step === 'code'}>Recovery code</li>
    </ol>

    {#if step === 'names'}
      <form class="stack" onsubmit={next}>
        <div class="field">
          <label for="shop">Shop name</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input id="shop" class="input" bind:value={shopName} placeholder="e.g. Main Street Repair" autofocus />
        </div>
        <div class="field">
          <label for="tech">Your name (lead technician)</label>
          <input id="tech" class="input" bind:value={techName} placeholder="e.g. Ana Ruiz" autocomplete="name" />
        </div>
        <p class="muted small">
          Technicians unlock admin actions with a PIN: adding tools, managing borrowers, inspections, reports and
          settings. You can add more technicians later.
        </p>
        {#if error}<p class="error-text">{error}</p>{/if}
        <button class="btn btn-primary btn-lg">Next</button>
        <button type="button" class="btn btn-ghost" onclick={ondemo} disabled={demoBusy}>
          <Icon name="box" /> {demoBusy ? 'Loading demo…' : 'Just looking? Try the demo with sample tools and people'}
        </button>
        {#if canShare}
          <button type="button" class="btn btn-ghost" onclick={pickShared}>
            <Icon name="database" /> Already set up in another account? Use its shared folder
          </button>
        {/if}
      </form>
    {:else if step === 'pin'}
      <PinCreate
        min={4}
        max={8}
        label="Choose your technician PIN"
        hint="6 digits recommended (4–8 allowed). Don't reuse a borrower PIN."
        onchosen={create}
      />
      <button class="btn btn-ghost" onclick={() => (step = 'names')}>Back</button>
    {:else}
      <h2>Your recovery code</h2>
      <RecoveryCode code={recoveryCode} ondone={finish} doneLabel="Start using ToolCheckout" />
    {/if}
  </div>
</div>

{#if shared?.existing}
  <ConfirmDialog title="Use this shared data?" confirmLabel="Use this data" onconfirm={useShared} onclose={() => (shared = null)}>
    <div class="stack">
      <code class="path">{shared.path}</code>
      <p>
        It holds <strong>{shared.existing.shopName}</strong> ({count(shared.existing.tools, 'tool')},
        {count(shared.existing.borrowers, 'borrower')}). This account will use it from now on, and technicians unlock with
        the PINs they already have.
      </p>
    </div>
  </ConfirmDialog>
{/if}

<style>
  .path {
    font-family: var(--mono);
    font-size: 0.88rem;
    padding: 0.35rem 0.6rem;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    word-break: break-all;
  }
  .setup {
    min-height: 100%;
    display: grid;
    place-items: center;
    padding: 2rem 1rem;
  }
  .panel {
    width: min(520px, 100%);
    padding: 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }
  .brand {
    display: flex;
    gap: 1rem;
    align-items: center;
  }
  .logo {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    border-radius: 14px;
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .steps {
    display: flex;
    gap: 0.5rem;
    list-style: none;
    padding: 0;
    margin: 0;
    counter-reset: s;
  }
  .steps li {
    counter-increment: s;
    flex: 1;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-faint);
    padding-top: 0.5rem;
    border-top: 3px solid var(--border);
  }
  .steps li::before {
    content: counter(s) '. ';
  }
  .steps li.on {
    color: var(--accent);
    border-color: var(--accent);
  }
  .small {
    font-size: 0.9rem;
  }
</style>
