<script lang="ts">
  // Checkout: tools (scan/search) → borrower (tap or badge) → PIN →
  // confirm each tool's condition and due-back → done.
  import { onMount } from 'svelte';
  import { api, asApiError, type BorrowerAuth, type BorrowerPick, type CheckoutReceipt, type ToolRow } from '../api';
  import { app } from '../app.svelte';
  import Avatar from '../components/Avatar.svelte';
  import ConditionBadge from '../components/ConditionBadge.svelte';
  import Icon from '../components/Icon.svelte';
  import Modal from '../components/Modal.svelte';
  import Photo from '../components/Photo.svelte';
  import PhotoInput from '../components/PhotoInput.svelte';
  import PinPad from '../components/PinPad.svelte';
  import ToolSearch from '../components/ToolSearch.svelte';
  import { loanPeriod, time, todayAt, when } from '../format';
  import { listenForScans } from '../scan';

  let {
    tools: initialTools,
    borrower: initialBorrower,
    onclose,
  }: { tools: ToolRow[]; borrower: BorrowerPick | null; onclose: () => void } = $props();

  type Due = 'default' | 'eod' | number;
  interface CartItem {
    tool: ToolRow;
    due: Due;
  }

  function start() {
    return { cart: initialTools.map((t) => ({ tool: t, due: 'default' as Due })), who: initialBorrower };
  }
  const init = start();
  let cart = $state<CartItem[]>(init.cart);
  let who = $state<BorrowerPick | null>(init.who);
  let people = $state<BorrowerPick[]>([]);
  let filter = $state('');
  let step = $state<'build' | 'pin' | 'confirm' | 'done'>('build');
  let auth = $state<BorrowerAuth | null>(null);
  let receipt = $state<CheckoutReceipt | null>(null);
  let reporting = $state<{ tool: ToolRow; notes: string; photoId: number | null; error: string | null } | null>(null);
  let busy = $state(false);
  let pending = $state('');
  let message = $state<string | null>(null);

  const closing = $derived(app.settings?.closingTime ?? '17:30');
  const pinRequired = $derived(app.settings?.borrowerPinRequired ?? true);
  const needsTech = $derived((app.settings?.techPinForCheckout ?? false) || cart.some((c) => c.tool.techOnly));
  const shown = $derived(
    filter.trim()
      ? people.filter((p) => `${p.name} ${p.employeeId}`.toLowerCase().includes(filter.trim().toLowerCase()))
      : people,
  );

  onMount(() => {
    api.borrowerPicker().then((p) => (people = p), app.error);
    const stop = listenForScans(onScan, (p) => (pending = p));
    maybeAdvance();
    return stop;
  });

  async function onScan(code: string) {
    if (step === 'done' || reporting) return;
    try {
      const r = await api.resolveScan(code);
      if (r.kind === 'tool') addTool(r.tool);
      else if (r.kind === 'borrower') pickBorrower(r.borrower);
      else app.toast(r.message, 'error');
    } catch (e) {
      app.error(e);
    }
  }

  function addTool(t: ToolRow) {
    if (cart.some((c) => c.tool.id === t.id)) return app.toast(`${t.assetTag} is already in this checkout`, 'info');
    if (t.status !== 'available') {
      const who = t.borrowerName ? ` (${t.borrowerName})` : '';
      return app.toast(`${t.assetTag} ${t.name} is ${t.status.replace(/_/g, ' ')}${who}`, 'error');
    }
    cart = [...cart, { tool: t, due: 'default' }];
    maybeAdvance();
  }

  function pickBorrower(b: BorrowerPick) {
    if (step === 'confirm' && auth && auth.borrower.id !== b.id) {
      auth = null; // a different person: they need their own PIN
    }
    who = b;
    if (step === 'confirm' && auth) return;
    step = 'build';
    maybeAdvance();
  }

  function maybeAdvance() {
    if (step === 'build' && who && cart.length) void toPin();
  }

  async function toPin() {
    message = null;
    if (!pinRequired) {
      try {
        auth = await api.authenticateBorrower(who!.id, null);
        step = 'confirm';
      } catch (e) {
        app.error(e);
      }
    } else {
      step = 'pin';
    }
  }

  async function submitPin(pin: string) {
    auth = await api.authenticateBorrower(who!.id, pin);
    step = 'confirm';
  }

  function dueAt(c: CartItem): number | null {
    if (c.due === 'default') return null;
    if (c.due === 'eod') return todayAt(closing, app.now());
    return app.now() + c.due * 60_000;
  }

  function dueText(c: CartItem): string {
    const at = dueAt(c) ?? app.now() + c.tool.defaultLoanMinutes * 60_000;
    return when(at, app.now());
  }

  async function confirm() {
    if (!auth || !cart.length) return;
    busy = true;
    const items = cart.map((c) => ({ toolId: c.tool.id, dueAt: dueAt(c) }));
    try {
      let r: CheckoutReceipt | undefined;
      try {
        r = await api.checkout(auth.token, items);
      } catch (e) {
        if (asApiError(e).code !== 'tech_required') throw e;
        const why = cart.some((c) => c.tool.techOnly)
          ? 'A tech-only tool is in this checkout. A technician must authorize it.'
          : 'This shop requires a technician PIN for every checkout.';
        if (!(await app.requestUnlock(why))) return;
        r = await api.checkout(auth.token, items);
      }
      receipt = r;
      step = 'done';
      setTimeout(() => step === 'done' && onclose(), 6000);
    } catch (e) {
      const err = asApiError(e);
      if (err.code === 'auth_expired') {
        auth = null;
        step = 'pin';
        message = 'That took a while — please enter your PIN again.';
      } else {
        app.error(err);
      }
    } finally {
      busy = false;
    }
  }

  async function sendReport(e: SubmitEvent) {
    e.preventDefault();
    if (!reporting || !auth) return;
    if (!reporting.notes.trim()) return void (reporting.error = 'Say briefly what is wrong');
    try {
      const t = reporting.tool;
      await api.reportProblem(auth.token, t.id, reporting.notes, reporting.photoId);
      cart = cart.filter((c) => c.tool.id !== t.id);
      reporting = null;
      app.toast(`${t.assetTag} flagged for a technician — thanks!`, 'info');
      if (!cart.length) onclose();
    } catch (err) {
      if (reporting) reporting.error = asApiError(err).message;
    }
  }

  const DUE_OPTIONS: { value: Due; label: (c: CartItem) => string }[] = [
    { value: 'default', label: (c) => `${loanPeriod(c.tool.defaultLoanMinutes)} (usual)` },
    { value: 30, label: () => '30 minutes' },
    { value: 60, label: () => '1 hour' },
    { value: 120, label: () => '2 hours' },
    { value: 240, label: () => '4 hours' },
    { value: 'eod', label: () => `End of day (${time(todayAt(closing, app.now()))})` },
    { value: 1440, label: () => '1 day' },
    { value: 4320, label: () => '3 days' },
    { value: 10080, label: () => '1 week' },
  ];
</script>

<Modal title="Check out" size="xl" {onclose}>
  {#if step === 'build'}
    <div class="cols">
      <section class="stack">
        <h3>1 · Tools</h3>
        <ToolSearch want={['available']} exclude={cart.map((c) => c.tool.id)} onpick={addTool} placeholder="Scan a tool tag or search…" />
        {#if cart.length}
          <ul class="cart">
            {#each cart as c (c.tool.id)}
              <li>
                <Photo id={c.tool.photoId} size={44} zoom={false} />
                <span class="grow"><strong>{c.tool.name}</strong> <span class="mono faint">{c.tool.assetTag}</span></span>
                {#if c.tool.techOnly}<span class="badge badge-neutral"><Icon name="lock" size={12} /> Tech-only</span>{/if}
                <button class="btn btn-ghost btn-icon" aria-label="Remove {c.tool.name}" onclick={() => (cart = cart.filter((x) => x !== c))}>
                  <Icon name="x" />
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="hint"><Icon name="scan" /> Scan the tool's tag, or search by name.</p>
        {/if}
      </section>
      <section class="stack">
        <h3>2 · Who's borrowing?</h3>
        <input class="input" type="search" placeholder="Find a name…" bind:value={filter} aria-label="Find borrower" />
        <div class="people">
          {#each shown as p (p.id)}
            <button class="person" class:on={who?.id === p.id} onclick={() => pickBorrower(p)}>
              <Avatar name={p.name} size={44} />
              <span class="pname">{p.name}</span>
              {#if p.employeeId}<span class="faint small">{p.employeeId}</span>{/if}
            </button>
          {:else}
            <p class="muted">No borrowers yet. A technician can add them under Borrowers.</p>
          {/each}
        </div>
        {#if who && !cart.length}<p class="hint">Now scan or search the tools {who.name} is taking.</p>{/if}
      </section>
    </div>
  {:else if step === 'pin' && who}
    <div class="pinstep">
      <Avatar name={who.name} size={64} />
      <h2>Hi {who.name.split(' ')[0]}!</h2>
      {#if message}<p class="error-text">{message}</p>{/if}
      <PinPad min={4} max={4} label="Enter your PIN to confirm it's you" onsubmit={submitPin} />
      <button class="btn btn-ghost" onclick={() => ((step = 'build'), (who = null))}>Not {who.name.split(' ')[0]}? Change</button>
    </div>
  {:else if step === 'confirm' && who}
    <div class="stack">
      <div class="row">
        <Avatar name={who.name} size={40} />
        <h2 class="grow">{who.name} is taking {cart.length} {cart.length === 1 ? 'tool' : 'tools'}</h2>
        <span class="faint small">Scan more tools to add them</span>
      </div>
      <p class="muted">Check each tool's condition below. By confirming, you agree it's in this condition now.</p>
      <ul class="confirm">
        {#each cart as c (c.tool.id)}
          <li class="card">
            <Photo id={c.tool.photoId} size={84} alt={c.tool.name} />
            <div class="grow stack tight">
              <div><strong>{c.tool.name}</strong> <span class="mono faint">{c.tool.assetTag}</span></div>
              <div class="row wrap">
                <ConditionBadge condition={c.tool.condition} />
                {#if c.tool.conditionNotes}<span class="muted">“{c.tool.conditionNotes}”</span>{/if}
              </div>
              <div class="row wrap">
                <label class="faint small" for="due-{c.tool.id}">Due back</label>
                <select id="due-{c.tool.id}" class="input due" bind:value={c.due}>
                  {#each DUE_OPTIONS as o (o.value)}<option value={o.value}>{o.label(c)}</option>{/each}
                </select>
                <span class="small">{dueText(c)}</span>
              </div>
            </div>
            <div class="side">
              {#if c.tool.techOnly}<span class="badge badge-neutral"><Icon name="lock" size={12} /> Tech authorizes</span>{/if}
              <button class="btn btn-sm report" onclick={() => (reporting = { tool: c.tool, notes: '', photoId: null, error: null })}>
                <Icon name="flag" size={16} /> Report a problem
              </button>
              <button class="btn btn-sm btn-ghost" onclick={() => (cart = cart.filter((x) => x !== c))}>Remove</button>
            </div>
          </li>
        {/each}
      </ul>
      {#if needsTech && !app.session}
        <p class="muted"><Icon name="lock" size={14} /> A technician will be asked for their PIN.</p>
      {/if}
    </div>
  {:else if step === 'done' && receipt}
    <div class="done">
      <span class="tick"><Icon name="check" size={40} stroke={3} /></span>
      <h2>All set, {receipt.borrower.name.split(' ')[0]}!</h2>
      <ul>
        {#each receipt.loans as l (l.id)}
          <li><strong>{l.toolName}</strong> <span class="mono faint">{l.assetTag}</span> — due back {when(l.dueAt, app.now())}</li>
        {/each}
      </ul>
      <button class="btn btn-primary btn-lg" onclick={onclose}>Done</button>
    </div>
  {/if}

  {#if pending && step !== 'done'}<div class="scanning mono" aria-live="polite">Reading: {pending}</div>{/if}

  {#snippet footer()}
    {#if step === 'confirm'}
      <button class="btn" onclick={() => (step = 'build')}>Back</button>
      <button class="btn btn-primary btn-lg confirm-btn" disabled={busy || !cart.length} onclick={confirm}>
        <Icon name="check" /> I confirm this condition — check out
      </button>
    {:else if step === 'build'}
      <button class="btn" onclick={onclose}>Cancel</button>
      <button class="btn btn-primary" disabled={!who || !cart.length} onclick={toPin}>Next</button>
    {/if}
  {/snippet}
</Modal>

{#if reporting}
  <Modal title="Report a problem — {reporting.tool.assetTag}" size="md" onclose={() => (reporting = null)}>
    <form id="reportform" class="stack" onsubmit={sendReport}>
      <p>
        This tool won't be lent. It goes to a technician for inspection, and the report is saved under your name — thanks for
        flagging it.
      </p>
      <div class="field">
        <label for="rp-notes">What's wrong?</label>
        <!-- svelte-ignore a11y_autofocus -->
        <textarea id="rp-notes" class="input" bind:value={reporting.notes} rows="3" autofocus placeholder="e.g. tip is chipped, battery dead"></textarea>
      </div>
      <PhotoInput bind:value={reporting.photoId} />
      {#if reporting.error}<p class="error-text">{reporting.error}</p>{/if}
    </form>
    {#snippet footer()}
      <button class="btn" onclick={() => (reporting = null)}>Cancel</button>
      <button form="reportform" class="btn btn-primary">Flag for inspection</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .cols {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
    min-height: 420px;
  }
  h3 {
    color: var(--text-muted);
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .cart,
  .confirm {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .cart li {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.4rem 0.4rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 1rem;
    border: 2px dashed var(--border-strong);
    border-radius: var(--radius-lg);
    color: var(--text-muted);
  }
  .people {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(118px, 1fr));
    gap: 0.5rem;
    max-height: 420px;
    overflow: auto;
    padding: 2px;
  }
  .person {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
    padding: 0.7rem 0.4rem;
    min-height: 104px;
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .person:hover {
    background: var(--surface-2);
  }
  .person.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .pname {
    font-weight: 650;
    text-align: center;
    line-height: 1.15;
  }
  .small {
    font-size: 0.82rem;
  }
  .pinstep {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0 1rem;
  }
  .confirm li {
    display: flex;
    gap: 1rem;
    padding: 0.8rem;
    align-items: flex-start;
  }
  .tight {
    gap: 0.35rem;
  }
  .due {
    width: auto;
    min-height: 40px;
    padding: 0.3rem 0.6rem;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    align-items: flex-end;
  }
  .report {
    color: var(--warning);
    border-color: var(--warning);
  }
  .confirm-btn {
    min-width: 360px;
  }
  .done {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.8rem;
    padding: 1.5rem 0;
    text-align: center;
  }
  .done ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .tick {
    display: grid;
    place-items: center;
    width: 76px;
    height: 76px;
    border-radius: 50%;
    background: var(--success-soft);
    color: var(--success);
  }
  .scanning {
    position: absolute;
    right: 16px;
    bottom: 76px;
    padding: 0.3rem 0.7rem;
    border-radius: var(--radius);
    background: var(--surface-3);
    font-size: 0.85rem;
  }
  @media (max-width: 900px) {
    .cols {
      grid-template-columns: 1fr;
    }
  }
</style>
