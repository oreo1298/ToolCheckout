<script lang="ts">
  import type { LoanRow } from '../api';
  import { dateTime, duration } from '../format';
  import ConditionBadge from './ConditionBadge.svelte';

  let { loans, show = 'borrower' }: { loans: LoanRow[]; show?: 'borrower' | 'tool' } = $props();

  const STATE: Record<LoanRow['state'], string> = {
    out: 'Out',
    awaiting_inspection: 'Dropped off',
    closed: 'Returned',
    lost: 'Lost',
  };
</script>

<div class="wrap">
  <table class="table">
    <thead>
      <tr>
        <th>Out</th>
        <th>{show === 'borrower' ? 'Borrower' : 'Tool'}</th>
        <th>Due</th>
        <th>Returned</th>
        <th>Condition</th>
        <th>Inspected by</th>
      </tr>
    </thead>
    <tbody>
      {#each loans as l (l.id)}
        <tr>
          <td class="nowrap">{dateTime(l.checkedOutAt)}</td>
          <td>
            {#if show === 'borrower'}{l.borrowerName}{:else}{l.toolName} <span class="mono faint">{l.assetTag}</span>{/if}
            {#if l.authorizedBy}<div class="faint small">authorized by {l.authorizedBy}</div>{/if}
          </td>
          <td class="nowrap">{dateTime(l.dueAt)}</td>
          <td>
            {#if l.returnedAt}{dateTime(l.returnedAt)}{:else}<span class="badge badge-{l.overdue ? 'danger' : l.state === 'lost' ? 'danger' : 'accent'}">{l.overdue ? 'Overdue' : STATE[l.state]}</span>{/if}
            {#if l.state === 'awaiting_inspection'}<div class="faint small">dropped off, awaiting inspection</div>{/if}
            {#if l.lateMs}<div class="late small">{duration(l.lateMs)} late</div>{/if}
          </td>
          <td>
            <span class="row"><ConditionBadge condition={l.checkoutCondition} />{#if l.returnCondition}→ <ConditionBadge condition={l.returnCondition} />{/if}</span>
            {#if l.returnNotes}<div class="faint small">“{l.returnNotes}”</div>{/if}
          </td>
          <td>{l.inspectedBy ?? '—'}</td>
        </tr>
      {:else}
        <tr><td colspan="6" class="empty">No loans yet</td></tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .wrap {
    overflow: auto;
    max-height: 420px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .small {
    font-size: 0.8rem;
  }
  .late {
    color: var(--danger);
    font-weight: 600;
  }
  td {
    font-size: 0.92rem;
  }
</style>
