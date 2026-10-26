<script lang="ts">
  import { api, type Incident, type IncidentKind, type Outcome } from '../api';
  import { app } from '../app.svelte';
  import ConditionBadge from '../components/ConditionBadge.svelte';
  import Icon from '../components/Icon.svelte';
  import Modal from '../components/Modal.svelte';
  import Photo from '../components/Photo.svelte';
  import TechGate from '../components/TechGate.svelte';
  import { INCIDENT_KINDS, OUTCOMES, outcomeLabel } from '../domain';
  import { dateTime, money } from '../format';

  let status = $state<'open' | 'resolved' | ''>('open');
  let kind = $state<IncidentKind | ''>('');
  let rows = $state<Incident[]>([]);
  let resolving = $state<Incident | null>(null);
  let outcome = $state<Outcome>('no_fault');
  let note = $state('');
  let error = $state<string | null>(null);

  $effect(() => {
    void app.dataVersion;
    if (!app.session) return;
    api.listIncidents({ status: status || undefined, kind: kind || undefined }).then((r) => (rows = r), app.error);
  });

  function open(i: Incident) {
    resolving = i;
    outcome = i.kind === 'lost' ? 'written_off' : i.kind === 'problem_report' ? 'inspected' : 'no_fault';
    note = '';
    error = null;
  }

  async function resolve(e: SubmitEvent) {
    e.preventDefault();
    if (!note.trim()) return void (error = 'Add a short note');
    try {
      const r = await app.withTech(() => api.resolveIncident(resolving!.id, outcome, note));
      if (r) {
        app.toast('Incident resolved');
        resolving = null;
      }
    } catch (err) {
      error = (err as { message: string }).message;
    }
  }
</script>

<TechGate title="Incidents">
  <div class="page">
    <h1>Incidents</h1>
    <div class="row wrap">
      <div class="seg" role="radiogroup" aria-label="Status">
        {#each [['open', 'Open'], ['resolved', 'Resolved'], ['', 'All']] as [v, l] (v)}
          <button role="radio" aria-checked={status === v} class:on={status === v} onclick={() => (status = v as typeof status)}>{l}</button>
        {/each}
      </div>
      <select class="input" style="width:auto" bind:value={kind} aria-label="Kind">
        <option value="">All kinds</option>
        {#each Object.entries(INCIDENT_KINDS) as [k, v] (k)}<option value={k}>{v.label}</option>{/each}
      </select>
    </div>

    {#each rows as i (i.id)}
      <article class="card inc" class:resolved={i.status === 'resolved'}>
        <div class="head row wrap">
          <span class="badge badge-{INCIDENT_KINDS[i.kind].tone}">{INCIDENT_KINDS[i.kind].label}</span>
          <button class="toolname" onclick={() => (app.toolDetailId = i.toolId)}>{i.toolName} <span class="mono faint">{i.assetTag}</span></button>
          <span class="spacer"></span>
          <span class="faint small">#{i.id} · {dateTime(i.createdAt)}</span>
        </div>
        <div class="body row wrap">
          <div class="grow stack tight">
            {#if i.borrowerName}
              <div>{i.kind === 'problem_report' ? 'Reported by' : 'Borrower'}: <strong>{i.borrowerName}</strong></div>
            {:else if i.kind !== 'problem_report'}
              <div class="muted">Not attributed to a borrower</div>
            {/if}
            {#if i.beforeCondition || i.afterCondition}
              <div class="row">
                {#if i.beforeCondition}<ConditionBadge condition={i.beforeCondition} />{/if}
                {#if i.afterCondition}<Icon name="chevronRight" size={16} /><ConditionBadge condition={i.afterCondition} />{/if}
              </div>
            {/if}
            {#if i.notes}<p>“{i.notes}”</p>{/if}
            <div class="faint small">
              Opened by {i.createdByName ?? i.createdByKind}
              {#if i.replacementCostCents != null && i.kind !== 'problem_report'}· replacement {money(i.replacementCostCents, app.settings?.currencySymbol)}{/if}
            </div>
            {#if i.status === 'resolved'}
              <div class="resolution">
                <strong>{outcomeLabel(i.outcome)}</strong> — {i.resolutionNote}
                <span class="faint small">({i.resolvedByName}, {dateTime(i.resolvedAt!)})</span>
              </div>
            {/if}
          </div>
          <div class="row">
            {#each i.photoIds as p (p)}<Photo id={p} size={72} alt="Incident photo" />{/each}
          </div>
          {#if i.status === 'open'}
            <button class="btn btn-primary" onclick={() => open(i)}>Resolve</button>
          {/if}
        </div>
      </article>
    {:else}
      <p class="empty card">{status === 'open' ? 'No open incidents. 🎉' : 'Nothing here.'}</p>
    {/each}
  </div>
</TechGate>

{#if resolving}
  <Modal title="Resolve incident #{resolving.id}" size="md" onclose={() => (resolving = null)}>
    <form id="resolveform" class="stack" onsubmit={resolve}>
      <p><strong>{INCIDENT_KINDS[resolving.kind].label}</strong> — {resolving.toolName}{resolving.borrowerName ? `, ${resolving.borrowerName}` : ''}</p>
      <div class="field">
        <label for="rs-out">Outcome</label>
        <select id="rs-out" class="input" bind:value={outcome}>
          {#each OUTCOMES as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
        </select>
      </div>
      <div class="field">
        <label for="rs-note">Note</label>
        <!-- svelte-ignore a11y_autofocus -->
        <textarea id="rs-note" class="input" bind:value={note} rows="3" autofocus placeholder="e.g. tip worn from normal use; no fault"></textarea>
      </div>
      {#if error}<p class="error-text">{error}</p>{/if}
    </form>
    {#snippet footer()}
      <button class="btn" onclick={() => (resolving = null)}>Cancel</button>
      <button form="resolveform" class="btn btn-primary">Resolve</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    max-width: 1100px;
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .seg button {
    min-height: 44px;
    padding: 0 1rem;
    border: 0;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .seg button + button {
    border-left: 1px solid var(--border-strong);
  }
  .seg button.on {
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .inc {
    padding: 0.9rem 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .inc.resolved {
    opacity: 0.85;
  }
  .body {
    align-items: flex-start;
    gap: 1rem;
  }
  .tight {
    gap: 0.3rem;
  }
  .toolname {
    border: 0;
    background: none;
    padding: 0;
    color: var(--text);
    font: inherit;
    font-weight: 650;
    cursor: pointer;
  }
  .small {
    font-size: 0.85rem;
  }
  .resolution {
    padding: 0.5rem 0.7rem;
    border-radius: var(--radius);
    background: var(--success-soft);
  }
  .empty {
    padding: 2rem;
    text-align: center;
    color: var(--text-faint);
  }
</style>
