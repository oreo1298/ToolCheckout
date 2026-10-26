<script lang="ts">
  // Add or edit a borrower; set/reset their PIN; activate/deactivate.
  import { api, type Borrower, type BorrowerDetail, type BorrowerInput } from '../api';
  import { app } from '../app.svelte';
  import Avatar from '../components/Avatar.svelte';
  import Icon from '../components/Icon.svelte';
  import LoanTable from '../components/LoanTable.svelte';
  import { INCIDENT_KINDS, outcomeLabel } from '../domain';
  import { dateTime, money } from '../format';
  import Modal from '../components/Modal.svelte';
  import PinCreate from '../components/PinCreate.svelte';

  let { borrower, onclose }: { borrower: Borrower | null; onclose: () => void } = $props();

  let detail = $state<BorrowerDetail | null>(null);
  $effect(() => {
    void app.dataVersion;
    if (borrower) api.borrowerDetail(borrower.id).then((d) => (detail = d), () => {});
  });

  function initial(): BorrowerInput & { active: boolean } {
    return {
      name: borrower?.name ?? '',
      employeeId: borrower?.employeeId ?? '',
      badgeCode: borrower?.badgeCode ?? '',
      active: borrower?.active ?? true,
    };
  }
  let form = $state(initial());
  let step = $state<'details' | 'pin'>('details');
  let error = $state<string | null>(null);
  let busy = $state(false);
  const isNew = $derived(borrower === null);
  const pinRequired = $derived(app.settings?.borrowerPinRequired ?? true);

  function input(): BorrowerInput {
    return { name: form.name, employeeId: form.employeeId, badgeCode: form.badgeCode || null };
  }

  async function saveDetails(e: SubmitEvent) {
    e.preventDefault();
    error = null;
    if (!form.name.trim()) return void (error = 'Name is required');
    if (isNew) {
      step = 'pin';
      return;
    }
    busy = true;
    try {
      const b = await app.withTech(() => api.updateBorrower(borrower!.id, input(), form.active));
      if (b) {
        app.toast(`Saved ${b.name}`);
        onclose();
      }
    } catch (err) {
      error = (err as { message: string }).message;
    } finally {
      busy = false;
    }
  }

  async function createWith(pin: string | null) {
    try {
      const b = await app.withTech(() => api.createBorrower(input(), pin));
      if (b) {
        app.toast(`Added ${b.name}`);
        onclose();
      }
    } catch (err) {
      // Duplicate badge etc. — back to the details form with the reason.
      step = 'details';
      error = (err as { message: string }).message;
    }
  }

  async function resetPin(pin: string) {
    await app.withTech(() => api.setBorrowerPin(borrower!.id, pin));
    app.toast(`PIN set for ${borrower!.name}`);
    step = 'details';
  }

  // Badge scanners end with Enter; keep that from submitting the form early.
  function badgeKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      (e.currentTarget as HTMLInputElement).blur();
    }
  }
</script>

<Modal title={isNew ? 'Add borrower' : borrower!.name} size="lg" {onclose}>
  {#if step === 'pin'}
    <div class="stack" style="align-items:center">
      <p class="muted center">
        {isNew ? `Ask ${form.name.trim()} to choose` : `Ask ${borrower?.name} to choose`} a 4-digit PIN. They'll enter it to
        confirm every checkout, so nobody can borrow in their name.
      </p>
      <PinCreate
        min={4}
        max={4}
        label="Choose a 4-digit PIN"
        onchosen={isNew ? (p) => createWith(p) : resetPin}
      />
      {#if isNew && !pinRequired}
        <button class="btn btn-ghost" onclick={() => createWith(null)}>Skip — PINs are turned off in Settings</button>
      {/if}
      <button class="btn btn-ghost" onclick={() => (step = 'details')}>Back</button>
    </div>
  {:else}
    <form id="bform" class="stack" onsubmit={saveDetails}>
      {#if !isNew}
        <div class="row">
          <Avatar name={borrower!.name} size={48} muted={!borrower!.active} />
          <div class="grow">
            <div class="row wrap">
              {#if borrower!.hasPin}<span class="badge badge-success">PIN set</span>{:else}<span class="badge badge-warning">No PIN</span>{/if}
              {#if !borrower!.active}<span class="badge badge-neutral">Inactive</span>{/if}
            </div>
          </div>
          <button type="button" class="btn btn-sm" onclick={() => (step = 'pin')}>
            <Icon name="key" size={16} />
            {borrower!.hasPin ? 'Reset PIN' : 'Set PIN'}
          </button>
        </div>
      {/if}
      <div class="grid">
        <div class="field span2">
          <label for="bf-name">Name</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input id="bf-name" class="input" bind:value={form.name} autofocus={isNew} placeholder="e.g. Jordan Lee" />
        </div>
        <div class="field">
          <label for="bf-emp">Employee ID (optional)</label>
          <input id="bf-emp" class="input" bind:value={form.employeeId} />
        </div>
        <div class="field">
          <label for="bf-badge">Badge barcode (optional)</label>
          <input
            id="bf-badge"
            class="input mono"
            bind:value={form.badgeCode}
            onkeydown={badgeKey}
            placeholder="Click here, then scan the badge"
            autocomplete="off"
          />
        </div>
      </div>
      {#if !isNew}
        <label class="check">
          <input type="checkbox" bind:checked={form.active} />
          Active (can borrow tools)
        </label>
      {/if}
      {#if error}<p class="error-text" role="alert">{error}</p>{/if}
    </form>
    {#if detail}
      <h3 class="sec">Loans ({detail.loans.length})</h3>
      <LoanTable loans={detail.loans} show="tool" />
      <h3 class="sec">Incidents ({detail.incidents.length})</h3>
      <ul class="incs">
        {#each detail.incidents as i (i.id)}
          <li>
            <span class="badge badge-{INCIDENT_KINDS[i.kind].tone}">{INCIDENT_KINDS[i.kind].label}</span>
            <span class="grow">
              {i.toolName} <span class="mono faint">{i.assetTag}</span> · {dateTime(i.createdAt)}
              {#if i.kind !== 'problem_report' && i.replacementCostCents != null}
                · {money(i.replacementCostCents, app.settings?.currencySymbol)}
              {/if}
            </span>
            {#if i.status === 'open'}<span class="badge badge-warning">Open</span>{:else}<span class="faint">{outcomeLabel(i.outcome)}</span>{/if}
          </li>
        {:else}
          <li class="muted">None.</li>
        {/each}
      </ul>
    {/if}
  {/if}
  {#snippet footer()}
    {#if step === 'details'}
      <button type="button" class="btn" onclick={onclose}>Cancel</button>
      <button form="bform" class="btn btn-primary" disabled={busy}>{isNew ? 'Next: PIN' : 'Save'}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.9rem 1rem;
  }
  .span2 {
    grid-column: 1 / -1;
  }
  .center {
    text-align: center;
    max-width: 420px;
  }
  .sec {
    margin: 1.2rem 0 0.5rem;
  }
  .incs {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .incs li {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    padding: 0.45rem 0;
    border-bottom: 1px solid var(--border);
  }
</style>
