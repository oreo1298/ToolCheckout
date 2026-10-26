<script lang="ts">
  import { api, type Borrower, type BorrowerRow } from '../api';
  import { app } from '../app.svelte';
  import Avatar from '../components/Avatar.svelte';
  import Icon from '../components/Icon.svelte';
  import TechGate from '../components/TechGate.svelte';
  import { when } from '../format';
  import BorrowerDialog from '../borrowers/BorrowerDialog.svelte';

  let search = $state('');
  let showInactive = $state(false);
  let rows = $state<BorrowerRow[]>([]);
  let open = $state<{ borrower: Borrower | null } | null>(null);

  $effect(() => {
    void app.dataVersion;
    if (!app.session) return;
    const f = { search: search || undefined, includeInactive: showInactive };
    const t = setTimeout(() => api.listBorrowers(f).then((r) => (rows = r), app.error), 120);
    return () => clearTimeout(t);
  });
</script>

<TechGate title="Borrowers">
  <div class="page">
    <div class="row wrap">
      <h1 class="grow">Borrowers</h1>
      <button class="btn btn-primary" onclick={() => (open = { borrower: null })}><Icon name="plus" /> Add borrower</button>
    </div>
    <div class="row wrap">
      <div class="search grow">
        <Icon name="search" />
        <input class="input" type="search" placeholder="Search name, employee ID, badge…" bind:value={search} aria-label="Search borrowers" />
      </div>
      <label class="check"><input type="checkbox" bind:checked={showInactive} /> Show inactive</label>
    </div>

    <div class="card list">
      <table class="table">
        <thead>
          <tr><th></th><th>Name</th><th>Employee ID</th><th>Badge</th><th>PIN</th><th>Tools out</th><th>Last borrowed</th></tr>
        </thead>
        <tbody>
          {#each rows as b (b.id)}
            <tr class="clickable" class:inactive={!b.active} onclick={() => (open = { borrower: b })}>
              <td style="width:52px"><Avatar name={b.name} size={36} muted={!b.active} /></td>
              <td>
                <button class="linkish" onclick={(e) => { e.stopPropagation(); open = { borrower: b }; }}>{b.name}</button>
                {#if !b.active}<span class="badge badge-neutral">Inactive</span>{/if}
              </td>
              <td>{b.employeeId || '—'}</td>
              <td class="mono small">{b.badgeCode ?? '—'}</td>
              <td>{#if b.hasPin}<span class="badge badge-success">Set</span>{:else}<span class="badge badge-warning">None</span>{/if}</td>
              <td>
                {b.openLoans}
                {#if b.overdueLoans}<span class="badge badge-danger">{b.overdueLoans} overdue</span>{/if}
              </td>
              <td class="small">{b.lastCheckoutAt ? when(b.lastCheckoutAt, app.nowMs) : '—'}</td>
            </tr>
          {:else}
            <tr><td colspan="7" class="empty">{search ? 'Nobody matches.' : 'No borrowers yet. Add the people who borrow your tools.'}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</TechGate>

{#if open}
  <BorrowerDialog borrower={open.borrower} onclose={() => (open = null)} />
{/if}

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .search {
    position: relative;
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
  .list {
    overflow: auto;
  }
  .small {
    font-size: 0.85rem;
  }
  tr.inactive td {
    color: var(--text-faint);
  }
  .linkish {
    border: 0;
    background: none;
    padding: 0;
    color: inherit;
    font: inherit;
    font-weight: 650;
    cursor: pointer;
  }
</style>
