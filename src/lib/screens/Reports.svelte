<script lang="ts">
  import {
    api,
    type BorrowerPick,
    type BorrowerSummary,
    type Incident,
    type LoanRow,
    type ReportFilter,
    type ReportKind,
    type ToolRow,
    type ToolSummary,
  } from '../api';
  import { app } from '../app.svelte';
  import ConditionBadge from '../components/ConditionBadge.svelte';
  import Icon from '../components/Icon.svelte';
  import LoanTable from '../components/LoanTable.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import TechGate from '../components/TechGate.svelte';
  import { CONDITIONS, INCIDENT_KINDS, STATUSES, outcomeLabel } from '../domain';
  import { date, dateTime, duration, money } from '../format';
  import { printWith } from '../print';
  import PrintTable from '../reports/PrintTable.svelte';

  type Tab = 'borrowers' | 'tools' | 'loans' | 'incidents';
  type Range = '7d' | '30d' | 'month' | 'lastmonth' | 'year' | 'all' | 'custom';

  let tab = $state<Tab>('borrowers');
  let range = $state<Range>('30d');
  let customFrom = $state('');
  let customTo = $state('');
  let borrowerId = $state<number | ''>('');
  let toolId = $state<number | ''>('');
  let people = $state<BorrowerPick[]>([]);
  let toolList = $state<ToolRow[]>([]);

  let borrowers = $state<BorrowerSummary[]>([]);
  let tools = $state<ToolSummary[]>([]);
  let loans = $state<LoanRow[]>([]);
  let incidents = $state<Incident[]>([]);

  const cur = $derived(app.settings?.currencySymbol ?? '$');

  function startOfDay(d: Date) {
    const x = new Date(d);
    x.setHours(0, 0, 0, 0);
    return x;
  }

  const filter = $derived.by((): ReportFilter => {
    const now = new Date(app.nowMs);
    const f: ReportFilter = {};
    const today = startOfDay(now);
    const day = 86_400_000;
    switch (range) {
      case '7d':
        f.from = today.getTime() - 6 * day;
        break;
      case '30d':
        f.from = today.getTime() - 29 * day;
        break;
      case 'month':
        f.from = new Date(now.getFullYear(), now.getMonth(), 1).getTime();
        break;
      case 'lastmonth':
        f.from = new Date(now.getFullYear(), now.getMonth() - 1, 1).getTime();
        f.to = new Date(now.getFullYear(), now.getMonth(), 1).getTime();
        break;
      case 'year':
        f.from = new Date(now.getFullYear(), 0, 1).getTime();
        break;
      case 'custom':
        if (customFrom) f.from = new Date(customFrom + 'T00:00').getTime();
        if (customTo) f.to = new Date(customTo + 'T00:00').getTime() + day;
        break;
    }
    if (borrowerId !== '') f.borrowerId = borrowerId;
    if (toolId !== '') f.toolId = toolId;
    return f;
  });

  const rangeText = $derived.by(() => {
    const parts: string[] = [];
    if (filter.from || filter.to) {
      parts.push(`${filter.from ? date(filter.from) : 'start'} – ${filter.to ? date(filter.to - 1) : 'today'}`);
    } else parts.push('All time');
    if (borrowerId !== '') parts.push(`Borrower: ${people.find((p) => p.id === borrowerId)?.name ?? ''}`);
    if (toolId !== '') parts.push(`Tool: ${toolList.find((t) => t.id === toolId)?.assetTag ?? ''}`);
    return parts.join(' · ');
  });

  $effect(() => {
    if (!app.session) return;
    void app.dataVersion;
    api.borrowerPicker().then((p) => (people = p), () => {});
    api.listTools({ includeRetired: true, limit: 5000 }).then((t) => (toolList = t), () => {});
  });

  $effect(() => {
    if (!app.session) return;
    void app.dataVersion;
    const f = $state.snapshot(filter) as ReportFilter;
    const t = tab;
    const load =
      t === 'borrowers'
        ? api.reportBorrowers(f).then((r) => (borrowers = r))
        : t === 'tools'
          ? api.reportTools(f).then((r) => (tools = r))
          : t === 'loans'
            ? api.reportLoans(f).then((r) => (loans = r))
            : api.reportIncidents(f, true).then((r) => (incidents = r));
    load.catch(app.error);
  });

  const KIND: Record<Tab, ReportKind> = { borrowers: 'borrowers', tools: 'tools', loans: 'loans', incidents: 'open_incidents' };
  const TITLE: Record<Tab, string> = {
    borrowers: 'Borrower summary',
    tools: 'Tool summary',
    loans: 'Loan history',
    incidents: 'Open incidents',
  };

  async function exportCsv() {
    const stamp = new Date().toISOString().slice(0, 10);
    try {
      const path = await app.withTech(() =>
        api.exportReportCsv(KIND[tab], $state.snapshot(filter) as ReportFilter, `toolcheckout-${KIND[tab]}-${stamp}.csv`),
      );
      if (path) app.toast(`Saved ${path}`);
    } catch (e) {
      app.error(e);
    }
  }

  function printable(): { headers: string[]; rows: string[][]; numeric: number[] } {
    switch (tab) {
      case 'borrowers':
        return {
          headers: ['Borrower', 'Checkouts', 'Late returns', 'Avg late', 'Out now', 'Overdue', 'Damage', 'Lost', 'Cost exposure'],
          rows: borrowers.map((b) => [
            b.name,
            String(b.checkouts),
            String(b.lateReturns),
            b.avgLateMs ? duration(b.avgLateMs) : '—',
            String(b.currentlyOut),
            String(b.currentlyOverdue),
            String(b.damageIncidents),
            String(b.lostTools),
            money(b.costExposureCents, cur),
          ]),
          numeric: [1, 2, 4, 5, 6, 7, 8],
        };
      case 'tools':
        return {
          headers: ['Tag', 'Tool', 'Status', 'Condition', 'Checkouts', 'Late', 'Time out', 'Damage', 'Lost', 'Replacement'],
          rows: tools.map((t) => [
            t.assetTag,
            t.name,
            t.status ? STATUSES[t.status].label : '',
            t.condition ? (CONDITIONS.find((c) => c.value === t.condition)?.label ?? '') : '',
            String(t.checkouts),
            String(t.lateReturns),
            duration(t.timeOutMs),
            String(t.damageIncidents),
            String(t.lostIncidents),
            money(t.replacementCostCents, cur),
          ]),
          numeric: [4, 5, 7, 8, 9],
        };
      case 'loans':
        return {
          headers: ['Out', 'Tag', 'Tool', 'Borrower', 'Due', 'Returned', 'Late', 'Condition'],
          rows: loans.map((l) => [
            dateTime(l.checkedOutAt),
            l.assetTag,
            l.toolName,
            l.borrowerName,
            dateTime(l.dueAt),
            l.returnedAt ? dateTime(l.returnedAt) : l.state === 'lost' ? 'Lost' : 'Still out',
            l.lateMs ? duration(l.lateMs) : '',
            `${l.checkoutCondition}${l.returnCondition ? ' → ' + l.returnCondition : ''}`,
          ]),
          numeric: [],
        };
      case 'incidents':
        return {
          headers: ['Opened', 'Kind', 'Tool', 'Borrower', 'Condition', 'Notes', 'Replacement'],
          rows: incidents.map((i) => [
            dateTime(i.createdAt),
            INCIDENT_KINDS[i.kind].label,
            `${i.assetTag} ${i.toolName}`,
            i.borrowerName ?? '',
            `${i.beforeCondition ?? ''}${i.afterCondition ? ' → ' + i.afterCondition : ''}`,
            i.notes,
            money(i.replacementCostCents, cur),
          ]),
          numeric: [6],
        };
    }
  }

  function print() {
    const p = printable();
    void printWith(
      PrintTable,
      { title: TITLE[tab], subtitle: rangeText, shopName: app.settings?.shopName ?? '', ...p },
      '@page { size: auto landscape; margin: 12mm; }',
    );
  }
</script>

<TechGate title="Reports">
  <div class="page">
    <div class="row wrap">
      <h1 class="grow">Reports</h1>
      <button class="btn" onclick={exportCsv}><Icon name="download" /> Export CSV</button>
      <button class="btn" onclick={print}><Icon name="printer" /> Print / PDF</button>
    </div>

    <div class="filters card card-pad row wrap">
      <div class="field">
        <label for="rp-range">Period</label>
        <select id="rp-range" class="input" bind:value={range}>
          <option value="7d">Last 7 days</option>
          <option value="30d">Last 30 days</option>
          <option value="month">This month</option>
          <option value="lastmonth">Last month</option>
          <option value="year">This year</option>
          <option value="all">All time</option>
          <option value="custom">Custom…</option>
        </select>
      </div>
      {#if range === 'custom'}
        <div class="field">
          <label for="rp-from">From</label>
          <input id="rp-from" class="input" type="date" bind:value={customFrom} />
        </div>
        <div class="field">
          <label for="rp-to">To</label>
          <input id="rp-to" class="input" type="date" bind:value={customTo} />
        </div>
      {/if}
      <div class="field">
        <label for="rp-b">Borrower</label>
        <select id="rp-b" class="input" bind:value={borrowerId}>
          <option value="">Everyone</option>
          {#each people as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
        </select>
      </div>
      <div class="field">
        <label for="rp-t">Tool</label>
        <select id="rp-t" class="input" bind:value={toolId}>
          <option value="">All tools</option>
          {#each toolList as t (t.id)}<option value={t.id}>{t.assetTag} · {t.name}</option>{/each}
        </select>
      </div>
    </div>

    <div class="tabs" role="tablist">
      {#each Object.entries(TITLE) as [k, v] (k)}
        <button role="tab" aria-selected={tab === k} class:on={tab === k} onclick={() => (tab = k as Tab)}>{v}</button>
      {/each}
    </div>
    <p class="faint small">{rangeText}{tab === 'incidents' ? ' (open incidents are shown whenever they were opened)' : ''}</p>

    <div class="card list">
      {#if tab === 'borrowers'}
        <table class="table">
          <thead>
            <tr>
              <th>Borrower</th><th class="num">Checkouts</th><th class="num">Late returns</th><th class="num">Avg lateness</th>
              <th class="num">Out now</th><th class="num">Damage</th><th class="num">Lost</th><th class="num">Problems reported</th>
              <th class="num">Cost of damaged/lost</th>
            </tr>
          </thead>
          <tbody>
            {#each borrowers as b (b.borrowerId)}
              <tr>
                <td><strong>{b.name}</strong></td>
                <td class="num">{b.checkouts}</td>
                <td class="num" title="{b.returned} returned">{b.lateReturns}</td>
                <td class="num">{b.avgLateMs ? duration(b.avgLateMs) : '—'}</td>
                <td class="num">{b.currentlyOut}{#if b.currentlyOverdue} <span class="badge badge-danger">{b.currentlyOverdue} overdue</span>{/if}</td>
                <td class="num">{b.damageIncidents}</td>
                <td class="num">{b.lostTools}</td>
                <td class="num">{b.problemsReported}</td>
                <td class="num" class:bad={b.costExposureCents > 0}>{money(b.costExposureCents, cur)}</td>
              </tr>
            {:else}
              <tr><td colspan="9" class="empty">No activity in this period.</td></tr>
            {/each}
          </tbody>
        </table>
        <p class="note faint">Cost counts damaged and lost tools' replacement cost unless the incident was resolved as “no fault” or “found”.</p>
      {:else if tab === 'tools'}
        <table class="table">
          <thead>
            <tr>
              <th>Tag</th><th>Tool</th><th>Status</th><th>Condition</th><th class="num">Checkouts</th><th class="num">Late</th>
              <th class="num">Time out</th><th class="num">Damage</th><th class="num">Lost</th><th class="num">Replacement</th>
            </tr>
          </thead>
          <tbody>
            {#each tools as t (t.toolId)}
              <tr class="clickable" onclick={() => (app.toolDetailId = t.toolId)}>
                <td class="mono">{t.assetTag}</td>
                <td>{t.name}</td>
                <td>{#if t.status}<StatusBadge status={t.status} />{/if}</td>
                <td>{#if t.condition}<ConditionBadge condition={t.condition} />{/if}</td>
                <td class="num">{t.checkouts}</td>
                <td class="num">{t.lateReturns}</td>
                <td class="num">{t.timeOutMs ? duration(t.timeOutMs) : '—'}</td>
                <td class="num">{t.damageIncidents}</td>
                <td class="num">{t.lostIncidents}</td>
                <td class="num">{money(t.replacementCostCents, cur)}</td>
              </tr>
            {:else}
              <tr><td colspan="10" class="empty">No tools.</td></tr>
            {/each}
          </tbody>
        </table>
      {:else if tab === 'loans'}
        <div class="pad">
          {#if toolId === ''}<p class="muted">Pick a tool above to see one tool's history, or browse all loans in the period.</p>{/if}
          <LoanTable {loans} show={toolId === '' ? 'tool' : 'borrower'} />
        </div>
      {:else}
        <table class="table">
          <thead><tr><th>Opened</th><th>Kind</th><th>Tool</th><th>Borrower</th><th>Notes</th><th class="num">Replacement</th></tr></thead>
          <tbody>
            {#each incidents as i (i.id)}
              <tr>
                <td class="nowrap">{dateTime(i.createdAt)}</td>
                <td><span class="badge badge-{INCIDENT_KINDS[i.kind].tone}">{INCIDENT_KINDS[i.kind].label}</span></td>
                <td>{i.toolName} <span class="mono faint">{i.assetTag}</span></td>
                <td>{i.borrowerName ?? '—'}</td>
                <td>{i.notes}{i.status === 'resolved' ? ` (${outcomeLabel(i.outcome)})` : ''}</td>
                <td class="num">{money(i.replacementCostCents, cur)}</td>
              </tr>
            {:else}
              <tr><td colspan="6" class="empty">No open incidents.</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </div>
</TechGate>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  .filters {
    align-items: flex-end;
    gap: 1rem;
  }
  .filters .field {
    min-width: 180px;
  }
  .tabs {
    display: flex;
    gap: 0.25rem;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    min-height: 46px;
    padding: 0 1rem;
    border: 0;
    border-bottom: 3px solid transparent;
    background: none;
    color: var(--text-muted);
    font: inherit;
    font-weight: 650;
    cursor: pointer;
  }
  .tabs button.on {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .list {
    overflow: auto;
  }
  .pad {
    padding: 0.75rem;
  }
  .small {
    font-size: 0.85rem;
  }
  .note {
    padding: 0.6rem 0.9rem;
    font-size: 0.85rem;
  }
  .bad {
    color: var(--danger);
    font-weight: 650;
  }
</style>
