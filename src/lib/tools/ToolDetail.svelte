<script lang="ts">
  // Everything about one tool: facts, current loan, actions, and history.
  import { api, type ToolDetail } from '../api';
  import { app } from '../app.svelte';
  import ConditionBadge from '../components/ConditionBadge.svelte';
  import Icon from '../components/Icon.svelte';
  import Modal from '../components/Modal.svelte';
  import Photo from '../components/Photo.svelte';
  import ReasonDialog from '../components/ReasonDialog.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import { CONDITION_SOURCES } from '../domain';
  import { dateTime, dueLabel, loanPeriod, money, when } from '../format';
  import ConditionDialog from './ConditionDialog.svelte';
  import ToolForm from './ToolForm.svelte';

  let { id, onclose }: { id: number; onclose: () => void } = $props();

  let detail = $state<ToolDetail | null>(null);
  let dialog = $state<
    null | 'edit' | 'duplicate' | 'condition' | 'repair-out' | 'repair-back' | 'retire' | 'reinstate'
  >(null);

  $effect(() => {
    void app.dataVersion;
    api.toolDetail(id).then(
      (d) => (detail = d),
      (e) => {
        app.error(e);
        onclose();
      },
    );
  });

  const t = $derived(detail?.row);
  const overdue = $derived(!!t?.dueAt && t.status === 'checked_out' && t.dueAt < app.nowMs);

  async function act<T>(fn: () => Promise<T>, msg: string) {
    const r = await app.withTech(fn);
    if (r !== undefined) {
      app.toast(msg);
      dialog = null;
    }
  }
</script>

<Modal title={t ? `${t.assetTag} · ${t.name}` : 'Tool'} size="xl" {onclose}>
  {#if t && detail}
    <div class="top">
      <Photo id={t.photoId} size={120} alt={t.name} />
      <div class="facts">
        <div class="row wrap">
          <StatusBadge status={t.status} {overdue} />
          <ConditionBadge condition={t.condition} />
          {#if t.techOnly}<span class="badge badge-neutral"><Icon name="lock" size={12} /> Tech-only</span>{/if}
        </div>
        {#if t.conditionNotes}<p class="muted">“{t.conditionNotes}”</p>{/if}
        <dl>
          <dt>Category</dt><dd>{t.category || '—'}</dd>
          <dt>Home</dt><dd>{t.homeLocation || '—'}</dd>
          <dt>Loan period</dt><dd>{loanPeriod(t.defaultLoanMinutes)}</dd>
          <dt>Replacement</dt><dd>{money(t.replacementCostCents, app.settings?.currencySymbol)}</dd>
          {#if t.model}<dt>Model</dt><dd>{t.model}</dd>{/if}
          {#if t.serial}<dt>Serial</dt><dd class="mono">{t.serial}</dd>{/if}
        </dl>
        {#if t.description}<p class="desc">{t.description}</p>{/if}
      </div>
      {#if t.borrowerName}
        <div class="loan card" class:overdue>
          <span class="label">With</span>
          <strong>{t.borrowerName}</strong>
          <span class="muted">since {when(t.checkedOutAt!, app.nowMs)}</span>
          {#if t.status === 'checked_out' && t.dueAt}
            <span class:error-text={overdue}>{dueLabel(t.dueAt, app.nowMs)}</span>
          {:else if t.status === 'awaiting_inspection'}
            <span>Dropped off — awaiting inspection</span>
          {/if}
        </div>
      {/if}
    </div>

    <div class="actions row wrap">
      <button class="btn btn-sm" onclick={() => (dialog = 'edit')}><Icon name="edit" size={16} /> Edit</button>
      <button class="btn btn-sm" onclick={() => (dialog = 'duplicate')}><Icon name="copy" size={16} /> Duplicate</button>
      {#if t.status === 'available' || t.status === 'out_for_repair'}
        <button class="btn btn-sm" onclick={() => (dialog = 'condition')}><Icon name="inspect" size={16} /> Update condition</button>
      {/if}
      {#if t.status === 'available'}
        <button class="btn btn-sm" onclick={() => (dialog = 'repair-out')}><Icon name="wrench" size={16} /> Send to repair</button>
      {/if}
      {#if t.status === 'out_for_repair'}
        <button class="btn btn-sm" onclick={() => (dialog = 'repair-back')}><Icon name="check" size={16} /> Back from repair</button>
      {/if}
      {#if t.status === 'available' || t.status === 'out_for_repair' || t.status === 'lost'}
        <button class="btn btn-sm" onclick={() => (dialog = 'retire')}><Icon name="archive" size={16} /> Retire</button>
      {/if}
      {#if t.status === 'retired'}
        <button class="btn btn-sm" onclick={() => (dialog = 'reinstate')}><Icon name="refresh" size={16} /> Reinstate</button>
      {/if}
    </div>

    <h3>Condition history</h3>
    <ol class="timeline">
      {#each detail.conditions as ev (ev.id)}
        <li>
          <div class="when faint">{dateTime(ev.createdAt)}</div>
          <div class="what">
            <div class="row wrap">
              <ConditionBadge condition={ev.condition} />
              <strong>{CONDITION_SOURCES[ev.source]}</strong>
              {#if ev.techName}<span class="muted">by {ev.techName}</span>{/if}
            </div>
            {#if ev.reason && ev.source === 'manual'}<div>Reason: {ev.reason}</div>{/if}
            {#if ev.notes}<div class="muted">“{ev.notes}”</div>{/if}
          </div>
          {#if ev.photoId}<Photo id={ev.photoId} size={48} alt="Condition photo" />{/if}
        </li>
      {/each}
    </ol>
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</Modal>

{#if t && dialog === 'edit'}
  <ToolForm mode="edit" source={t} onclose={() => (dialog = null)} onsaved={() => (dialog = null)} />
{:else if t && dialog === 'duplicate'}
  <ToolForm
    mode="duplicate"
    source={t}
    onclose={() => (dialog = null)}
    onsaved={(n) => {
      dialog = null;
      app.toolDetailId = n.id;
    }}
  />
{:else if t && dialog === 'condition'}
  <ConditionDialog tool={t} mode="update" onclose={() => (dialog = null)} />
{:else if t && dialog === 'repair-back'}
  <ConditionDialog tool={t} mode="repair" onclose={() => (dialog = null)} />
{:else if t && dialog === 'reinstate'}
  <ConditionDialog tool={t} mode="reinstate" onclose={() => (dialog = null)} />
{:else if t && dialog === 'repair-out'}
  <ReasonDialog
    title="Send {t.assetTag} to repair"
    label="What needs fixing?"
    confirm="Send to repair"
    onclose={() => (dialog = null)}
    onsubmit={(r) => act(() => api.sendToolToRepair(t.id, r), `${t.name} is out for repair`)}
  />
{:else if t && dialog === 'retire'}
  <ReasonDialog
    title="Retire {t.assetTag}"
    label="Why is it being retired?"
    message="Retired tools stay in the history and reports but can't be loaned. The tag is never reused."
    confirm="Retire tool"
    danger
    onclose={() => (dialog = null)}
    onsubmit={(r) => act(() => api.retireTool(t.id, r), `${t.name} retired`)}
  />
{/if}

<style>
  .top {
    display: flex;
    gap: 1.25rem;
    align-items: flex-start;
    flex-wrap: wrap;
  }
  .facts {
    flex: 1;
    min-width: 260px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.2rem 1rem;
    margin: 0;
  }
  dt {
    color: var(--text-muted);
    font-size: 0.9rem;
  }
  dd {
    margin: 0;
  }
  .desc {
    white-space: pre-wrap;
  }
  .loan {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.9rem 1rem;
    min-width: 220px;
    border-left: 4px solid var(--accent);
  }
  .loan.overdue {
    border-left-color: var(--danger);
  }
  .actions {
    margin: 1rem 0;
    padding: 0.75rem 0;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }
  h3 {
    margin-bottom: 0.5rem;
  }
  .timeline {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .timeline li {
    display: flex;
    gap: 1rem;
    padding: 0.6rem 0;
    border-bottom: 1px solid var(--border);
    align-items: flex-start;
  }
  .when {
    width: 170px;
    flex: none;
    font-size: 0.9rem;
  }
  .what {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
</style>
