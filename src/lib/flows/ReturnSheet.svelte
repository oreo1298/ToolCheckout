<script lang="ts">
  // Return: pick the tools coming back, then either a technician inspects them
  // now, or the borrower drops them off for inspection with their PIN.
  import { onMount } from 'svelte';
  import { api, asApiError, type Condition, type InspectionResult, type LoanRow, type ToolRow } from '../api';
  import { app } from '../app.svelte';
  import Avatar from '../components/Avatar.svelte';
  import ConditionBadge from '../components/ConditionBadge.svelte';
  import Icon from '../components/Icon.svelte';
  import Modal from '../components/Modal.svelte';
  import Photo from '../components/Photo.svelte';
  import PinPad from '../components/PinPad.svelte';
  import ToolSearch from '../components/ToolSearch.svelte';
  import { dueLabel, when } from '../format';
  import { INCIDENT_KINDS } from '../domain';
  import { listenForScans } from '../scan';
  import InspectForm from './InspectForm.svelte';

  let { loans: initial, onclose }: { loans: LoanRow[]; onclose: () => void } = $props();

  interface Item {
    loan: LoanRow;
    condition: Condition | null;
    notes: string;
    photoId: number | null;
  }
  const mk = (l: LoanRow): Item => ({ loan: l, condition: null, notes: '', photoId: null });
  const start = () => initial.map(mk);
  let items = $state<Item[]>(start());
  let stillOut = $state<LoanRow[]>([]);
  let step = $state<'pick' | 'inspect' | 'dropoff' | 'done'>('pick');
  let results = $state<InspectionResult[]>([]);
  let droppedOff = $state<LoanRow[]>([]);
  let dropNote = $state('');
  let busy = $state(false);
  let pending = $state('');
  let error = $state<string | null>(null);

  // Drop-off goes borrower by borrower (each confirms their own tools).
  const groups = $derived.by(() => {
    const m = new Map<number, { id: number; name: string; ids: number[] }>();
    for (const it of items) {
      const g = m.get(it.loan.borrowerId) ?? { id: it.loan.borrowerId, name: it.loan.borrowerName, ids: [] };
      g.ids.push(it.loan.id);
      m.set(it.loan.borrowerId, g);
    }
    return [...m.values()];
  });
  let groupIdx = $state(0);

  onMount(() => {
    api.home().then((h) => (stillOut = [...h.overdue, ...h.out]), app.error);
    return listenForScans(onScan, (p) => (pending = p));
  });

  async function onScan(code: string) {
    if (step !== 'pick') return;
    try {
      const r = await api.resolveScan(code);
      if (r.kind === 'tool') await addTool(r.tool, r.loan);
      else if (r.kind === 'borrower') {
        const theirs = stillOut.filter((l) => l.borrowerId === r.borrower.id);
        if (!theirs.length) app.toast(`${r.borrower.name} has nothing checked out`, 'info');
        theirs.forEach(add);
      } else app.toast(r.message, 'error');
    } catch (e) {
      app.error(e);
    }
  }

  async function addTool(t: ToolRow, loan?: LoanRow | null) {
    if (t.status === 'awaiting_inspection') {
      return app.toast(`${t.assetTag} was already dropped off — inspect it from “Awaiting inspection”.`, 'info');
    }
    if (t.status !== 'checked_out') return app.toast(`${t.assetTag} ${t.name} isn't checked out`, 'error');
    const l = loan ?? (await api.openLoanForTool(t.id));
    if (l) add(l);
  }

  function add(l: LoanRow) {
    if (!items.some((i) => i.loan.id === l.id)) items = [...items, mk(l)];
  }

  async function inspectAll() {
    error = null;
    if (items.some((i) => !i.condition)) return void (error = 'Choose a condition for every tool');
    busy = true;
    try {
      const done = await app.withTech(async () => {
        const out: InspectionResult[] = [];
        for (const it of items) {
          out.push(await api.inspectReturn(it.loan.id, it.condition!, it.notes, it.photoId));
        }
        return out;
      }, 'Return inspection needs a technician PIN.');
      if (done) {
        results = done;
        step = 'done';
      }
    } catch (e) {
      error = asApiError(e).message;
    } finally {
      busy = false;
    }
  }

  async function startDropoff() {
    groupIdx = 0;
    step = 'dropoff';
    if (!(app.settings?.borrowerPinRequired ?? true)) await dropFor(null);
  }

  async function dropFor(pin: string | null) {
    const g = groups[groupIdx];
    const auth = await api.authenticateBorrower(g.id, pin);
    const done = await api.dropOff(auth.token, g.ids, dropNote);
    droppedOff = [...droppedOff, ...done];
    if (groupIdx + 1 < groups.length) {
      groupIdx++;
      if (!(app.settings?.borrowerPinRequired ?? true)) await dropFor(null);
    } else {
      step = 'done';
    }
  }
</script>

<Modal title="Return tools" size="xl" {onclose}>
  {#if step === 'pick'}
    <div class="cols">
      <section class="stack">
        <h3>Returning</h3>
        <ToolSearch want={['checked_out']} onpick={(t) => addTool(t)} exclude={items.map((i) => i.loan.toolId)} placeholder="Scan a tool tag or search…" />
        {#if items.length}
          <ul class="list">
            {#each items as it (it.loan.id)}
              {@const l = it.loan}
              <li class="card">
                <Photo id={l.toolPhotoId} size={52} zoom={false} />
                <div class="grow">
                  <div><strong>{l.toolName}</strong> <span class="mono faint">{l.assetTag}</span></div>
                  <div class="small muted row wrap">
                    <Avatar name={l.borrowerName} size={20} /> {l.borrowerName} · out since {when(l.checkedOutAt, app.nowMs)}
                  </div>
                  <div class="small row wrap">
                    At checkout: <ConditionBadge condition={l.checkoutCondition} />
                    <span class:error-text={l.overdue}>{dueLabel(l.dueAt, app.nowMs)}</span>
                  </div>
                </div>
                <button class="btn btn-ghost btn-icon" aria-label="Remove" onclick={() => (items = items.filter((x) => x !== it))}><Icon name="x" /></button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="hint"><Icon name="scan" /> Scan the tools being returned, or tap them on the right.</p>
        {/if}
      </section>
      <section class="stack">
        <h3>Still out</h3>
        <ul class="list compact">
          {#each stillOut.filter((l) => !items.some((i) => i.loan.id === l.id)) as l (l.id)}
            <li>
              <button class="pick" onclick={() => add(l)}>
                <span class="grow"><strong>{l.toolName}</strong> <span class="mono faint">{l.assetTag}</span></span>
                <span class="small">{l.borrowerName}</span>
                {#if l.overdue}<span class="badge badge-danger">Overdue</span>{/if}
              </button>
            </li>
          {:else}
            <li class="muted">Nothing else is checked out.</li>
          {/each}
        </ul>
      </section>
    </div>
  {:else if step === 'inspect'}
    <div class="stack">
      {#each items as it, i (it.loan.id)}
        <section class="card card-pad stack">
          <div class="row">
            <Photo id={it.loan.toolPhotoId} size={48} />
            <h3 class="grow plain">{i + 1}. {it.loan.toolName} <span class="mono faint">{it.loan.assetTag}</span></h3>
            <span class="muted small">from {it.loan.borrowerName}</span>
          </div>
          <InspectForm before={it.loan.checkoutCondition} borrowerName={it.loan.borrowerName} bind:condition={it.condition} bind:notes={it.notes} bind:photoId={it.photoId} />
        </section>
      {/each}
      {#if error}<p class="error-text" role="alert">{error}</p>{/if}
    </div>
  {:else if step === 'dropoff'}
    {@const g = groups[groupIdx]}
    <div class="pinstep">
      <Avatar name={g.name} size={60} />
      <h2>{g.name}, drop off {g.ids.length} {g.ids.length === 1 ? 'tool' : 'tools'}</h2>
      <p class="muted center">
        Leave them on the inspection shelf. They stay under your name until a technician checks them.
      </p>
      <input class="input note" bind:value={dropNote} placeholder="Anything to report? (optional)" data-no-scan />
      {#key groupIdx}
        <PinPad min={4} max={4} label="Enter your PIN" onsubmit={(p) => dropFor(p)} />
      {/key}
      {#if groups.length > 1}<p class="faint">{groupIdx + 1} of {groups.length} people</p>{/if}
    </div>
  {:else}
    <div class="done">
      <span class="tick"><Icon name="check" size={40} stroke={3} /></span>
      {#if droppedOff.length}
        <h2>Dropped off for inspection</h2>
        <p class="muted">{droppedOff.length} {droppedOff.length === 1 ? 'tool is' : 'tools are'} waiting for a technician.</p>
      {:else}
        <h2>Returned and inspected</h2>
        <ul class="summary">
          {#each results as r (r.tool.id)}
            <li>
              <strong>{r.tool.name}</strong> → {r.tool.status === 'out_for_repair' ? 'out for repair' : 'back on the shelf'}
              {#if r.incident}<span class="badge badge-{INCIDENT_KINDS[r.incident.kind].tone}">Damage incident opened</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
      <button class="btn btn-primary btn-lg" onclick={onclose}>Done</button>
    </div>
  {/if}

  {#if pending && step === 'pick'}<div class="scanning mono">Reading: {pending}</div>{/if}

  {#snippet footer()}
    {#if step === 'pick'}
      <button class="btn" onclick={onclose}>Cancel</button>
      <span class="spacer"></span>
      <button class="btn btn-lg" disabled={!items.length} onclick={startDropoff}>
        <Icon name="inspect" /> Drop off for inspection
      </button>
      <button class="btn btn-primary btn-lg" disabled={!items.length} onclick={() => (step = 'inspect')}>
        <Icon name="check" /> Inspect now (technician)
      </button>
    {:else if step === 'inspect'}
      <button class="btn" onclick={() => (step = 'pick')}>Back</button>
      <button class="btn btn-primary btn-lg" disabled={busy} onclick={inspectAll}>Complete inspection</button>
    {:else if step === 'dropoff'}
      <button class="btn" onclick={() => (step = 'pick')}>Back</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .cols {
    display: grid;
    grid-template-columns: 1.2fr 1fr;
    gap: 1.5rem;
    min-height: 400px;
  }
  h3 {
    color: var(--text-muted);
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  h3.plain {
    color: var(--text);
    font-size: 1.05rem;
    text-transform: none;
    letter-spacing: 0;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .list > li.card {
    display: flex;
    gap: 0.75rem;
    padding: 0.6rem;
    align-items: center;
  }
  .compact {
    max-height: 440px;
    overflow: auto;
    gap: 0.3rem;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    min-height: 48px;
    padding: 0.3rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .pick:hover {
    background: var(--surface-2);
  }
  .small {
    font-size: 0.85rem;
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
  .pinstep {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
  }
  .center {
    text-align: center;
    max-width: 460px;
  }
  .note {
    max-width: 420px;
  }
  .done {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.8rem;
    padding: 1.5rem 0;
    text-align: center;
  }
  .summary {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
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
