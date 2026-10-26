<script lang="ts">
  import { api, type ToolRow, type ToolStatus } from '../api';
  import { app } from '../app.svelte';
  import ConditionBadge from '../components/ConditionBadge.svelte';
  import Icon from '../components/Icon.svelte';
  import Photo from '../components/Photo.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import TechGate from '../components/TechGate.svelte';
  import { STATUSES } from '../domain';
  import ToolForm from '../tools/ToolForm.svelte';
  import LabelsDialog from '../labels/LabelsDialog.svelte';

  let search = $state('');
  let status = $state<ToolStatus | ''>('');
  let category = $state('');
  let rows = $state<ToolRow[]>([]);
  let categories = $state<string[]>([]);
  let adding = $state(false);
  let labels = $state(false);

  $effect(() => {
    void app.dataVersion;
    const f = { search: search || undefined, status: status || undefined, category: category || undefined };
    const t = setTimeout(() => {
      api.listTools(f).then((r) => (rows = r), app.error);
    }, 120);
    return () => clearTimeout(t);
  });

  $effect(() => {
    void app.dataVersion;
    api.toolCategories().then((c) => (categories = c), () => {});
  });
</script>

<TechGate title="Tools">
  <div class="page">
    <div class="row wrap head">
      <h1 class="grow">Tools</h1>
      <button class="btn" onclick={() => (labels = true)}><Icon name="tag" /> Print labels</button>
      <button class="btn btn-primary" onclick={() => (adding = true)}><Icon name="plus" /> Add tool</button>
    </div>

    <div class="filters row wrap">
      <div class="search">
        <Icon name="search" />
        <input class="input" type="search" placeholder="Search name, tag, location, serial, borrower…" bind:value={search} aria-label="Search tools" />
      </div>
      <select class="input" bind:value={category} aria-label="Category" style="width:auto">
        <option value="">All categories</option>
        {#each categories as c (c)}<option value={c}>{c}</option>{/each}
      </select>
    </div>
    <div class="chips row wrap" role="radiogroup" aria-label="Status">
      <button class="chip" class:on={status === ''} role="radio" aria-checked={status === ''} onclick={() => (status = '')}>All active</button>
      {#each Object.entries(STATUSES) as [s, info] (s)}
        <button class="chip" class:on={status === s} role="radio" aria-checked={status === s} onclick={() => (status = s as ToolStatus)}>{info.label}</button>
      {/each}
    </div>

    <div class="card list">
      <table class="table">
        <thead>
          <tr><th></th><th>Tag</th><th>Tool</th><th>Home</th><th>Status</th><th>Condition</th></tr>
        </thead>
        <tbody>
          {#each rows as r (r.id)}
            <tr class="clickable" onclick={() => (app.toolDetailId = r.id)}>
              <td style="width:56px"><Photo id={r.photoId} size={40} zoom={false} alt={r.name} /></td>
              <td class="mono nowrap">{r.assetTag}</td>
              <td>
                <button class="linkish" onclick={(e) => { e.stopPropagation(); app.toolDetailId = r.id; }}>{r.name}</button>
                {#if r.techOnly}<Icon name="lock" size={13} label="Tech-only" />{/if}
                <div class="faint small">{r.category}</div>
              </td>
              <td class="small">{r.homeLocation}</td>
              <td>
                <StatusBadge status={r.status} overdue={r.status === 'checked_out' && !!r.dueAt && r.dueAt < app.nowMs} />
                {#if r.borrowerName}<div class="small muted">{r.borrowerName}</div>{/if}
              </td>
              <td><ConditionBadge condition={r.condition} /></td>
            </tr>
          {:else}
            <tr>
              <td colspan="6" class="empty">
                {#if search || status || category}No tools match.{:else}No tools yet. Add your first tool to get started.{/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</TechGate>

{#if labels}
  <LabelsDialog onclose={() => (labels = false)} />
{/if}

{#if adding}
  <ToolForm
    mode="add"
    onclose={() => (adding = false)}
    onsaved={(t) => {
      adding = false;
      app.toolDetailId = t.id;
    }}
  />
{/if}

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .search {
    position: relative;
    flex: 1;
    min-width: 260px;
  }
  .search :global(.icon) {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-faint);
  }
  .search .input {
    padding-left: 40px;
  }
  .chip {
    min-height: 38px;
    padding: 0 0.9rem;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
  }
  .chip.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast);
  }
  .list {
    overflow: auto;
  }
  .small {
    font-size: 0.85rem;
  }
  .linkish {
    border: 0;
    background: none;
    padding: 0;
    color: var(--text);
    font: inherit;
    font-weight: 650;
    cursor: pointer;
    text-align: left;
  }
</style>
