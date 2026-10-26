<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type AwaitingItem, type BorrowerPick, type Home, type LoanRow, type ScanResult, type ToolRow } from '../api';
  import { app } from '../app.svelte';
  import Avatar from '../components/Avatar.svelte';
  import Icon from '../components/Icon.svelte';
  import Photo from '../components/Photo.svelte';
  import { STATUSES } from '../domain';
  import { duration, when } from '../format';
  import { listenForScans } from '../scan';

  let home = $state<Home | null>(null);
  let q = $state('');
  let toolHits = $state<ToolRow[]>([]);
  let people = $state<BorrowerPick[]>([]);
  let pending = $state('');

  const peopleHits = $derived(
    q.trim().length >= 2 ? people.filter((p) => p.name.toLowerCase().includes(q.trim().toLowerCase())).slice(0, 5) : [],
  );

  async function load() {
    try {
      home = await api.home();
    } catch (e) {
      app.error(e);
    }
  }

  $effect(() => {
    void app.dataVersion;
    void load();
    api.borrowerPicker().then((p) => (people = p), () => {});
  });

  onMount(() => {
    // Overdue status changes with time, not just with data.
    const t = setInterval(load, 30_000);
    const stop = listenForScans(
      (code) => void scanned(code),
      (p) => (pending = p),
      // Sheets and dialogs on top do their own scanning.
      () => !app.flow && !app.unlockRequest && app.toolDetailId === null,
    );
    return () => {
      clearInterval(t);
      stop();
    };
  });

  $effect(() => {
    const query = q.trim();
    if (!query) return void (toolHits = []);
    const t = setTimeout(() => api.listTools({ search: query, limit: 8 }).then((r) => (toolHits = r)), 120);
    return () => clearTimeout(t);
  });

  async function scanned(code: string) {
    try {
      act(await api.resolveScan(code));
    } catch (e) {
      app.error(e);
    }
  }

  /** Scan-to-act: do the obvious thing for whatever was scanned. */
  function act(r: ScanResult) {
    q = '';
    if (r.kind === 'borrower') {
      app.flow = { kind: 'checkout', tools: [], borrower: r.borrower };
    } else if (r.kind === 'tool') {
      openTool(r.tool, r.loan);
    } else {
      app.toast(r.message, 'error');
    }
  }

  async function openTool(t: ToolRow, loan?: LoanRow | null) {
    q = '';
    switch (t.status) {
      case 'available':
        app.flow = { kind: 'checkout', tools: [t], borrower: null };
        break;
      case 'checked_out': {
        const l = loan ?? (await api.openLoanForTool(t.id));
        if (l) app.flow = { kind: 'return', loans: [l] };
        break;
      }
      case 'awaiting_inspection': {
        const item = home?.awaiting.find((a) => a.toolId === t.id);
        if (item) app.flow = { kind: 'inspect', item };
        break;
      }
      default:
        app.toast(`${t.assetTag} ${t.name} is ${STATUSES[t.status].label.toLowerCase()}.`, 'info');
        if (app.session) app.toolDetailId = t.id;
    }
  }

  async function searchEnter(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      q = '';
      return;
    }
    if (e.key !== 'Enter' || !q.trim()) return;
    e.preventDefault();
    const r = await api.resolveScan(q);
    if (r.kind !== 'unknown') act(r);
    else if (toolHits.length === 1 && !peopleHits.length) openTool(toolHits[0]);
    else if (peopleHits.length === 1 && !toolHits.length) act({ kind: 'borrower', borrower: peopleHits[0] });
  }

  function inspect(a: AwaitingItem) {
    app.flow = { kind: 'inspect', item: a };
  }

  const overdueCount = $derived(home?.overdue.length ?? 0);

  // After closing time, remind whoever is at the counter what's still out.
  const pastClosing = $derived.by(() => {
    const s = app.settings;
    if (!s?.endOfDayReminder) return false;
    const [h, m] = s.closingTime.split(':').map(Number);
    const closing = new Date(app.nowMs);
    closing.setHours(h, m, 0, 0);
    return app.nowMs >= closing.getTime();
  });
  const stillOut = $derived(home ? [...home.overdue, ...home.out] : []);
</script>

<div class="home">
  <div class="actions">
    <button class="big checkout" onclick={() => (app.flow = { kind: 'checkout', tools: [], borrower: null })}>
      <Icon name="out" size={34} />
      <span>Check out</span>
    </button>
    <button class="big return" onclick={() => (app.flow = { kind: 'return', loans: [] })}>
      <Icon name="in" size={34} />
      <span>Return</span>
    </button>
  </div>

  <div class="search">
    <Icon name="scan" size={22} />
    <input
      class="input"
      type="search"
      bind:value={q}
      onkeydown={searchEnter}
      placeholder="Scan anywhere, or type a tool, tag or person…"
      aria-label="Search tools and people"
    />
    {#if pending}<span class="reading mono">Reading: {pending}</span>{/if}
    {#if toolHits.length || peopleHits.length}
      <ul class="hits card">
        {#each peopleHits as p (p.id)}
          <li>
            <button onclick={() => act({ kind: 'borrower', borrower: p })}>
              <Avatar name={p.name} size={32} />
              <span class="grow"><strong>{p.name}</strong> <span class="faint">— check out to</span></span>
            </button>
          </li>
        {/each}
        {#each toolHits as t (t.id)}
          <li>
            <button onclick={() => openTool(t)}>
              <Photo id={t.photoId} size={32} zoom={false} />
              <span class="grow"><strong>{t.name}</strong> <span class="mono faint">{t.assetTag}</span></span>
              <span class="badge badge-{STATUSES[t.status].tone}">{STATUSES[t.status].label}</span>
              {#if t.borrowerName}<span class="faint small">{t.borrowerName}</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if home}
    {#if pastClosing && stillOut.length}
      <div class="eod" role="status">
        <Icon name="bell" />
        <span>
          <strong>End of day:</strong>
          {stillOut.length} {stillOut.length === 1 ? 'tool is' : 'tools are'} still out —
          {[...new Set(stillOut.map((l) => l.borrowerName))].join(', ')}.
        </span>
      </div>
    {/if}
    {#if overdueCount}
      <section class="panel overdue" aria-labelledby="ov-h">
        <h2 id="ov-h"><Icon name="alert" /> Overdue <span class="count">{overdueCount}</span></h2>
        <ul>
          {#each home.overdue as l (l.id)}
            <li>
              <button class="row-btn" onclick={() => (app.flow = { kind: 'return', loans: [l] })}>
                <Avatar name={l.borrowerName} size={36} />
                <span class="who"><strong>{l.borrowerName}</strong><span class="faint small">since {when(l.checkedOutAt, app.nowMs)}</span></span>
                <span class="what grow"><strong>{l.toolName}</strong> <span class="mono faint">{l.assetTag}</span></span>
                <span class="late">{duration(app.nowMs - l.dueAt)} overdue</span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <div class="panels">
      <section class="panel" aria-labelledby="out-h">
        <h2 id="out-h">Currently out <span class="count">{home.out.length}</span></h2>
        <ul>
          {#each home.out as l (l.id)}
            <li>
              <button class="row-btn" onclick={() => (app.flow = { kind: 'return', loans: [l] })}>
                <Avatar name={l.borrowerName} size={32} />
                <span class="who"><strong>{l.borrowerName}</strong><span class="faint small">{duration(app.nowMs - l.checkedOutAt)} out</span></span>
                <span class="what grow"><strong>{l.toolName}</strong> <span class="mono faint">{l.assetTag}</span></span>
                <span class="due small">due {when(l.dueAt, app.nowMs)}</span>
              </button>
            </li>
          {:else}
            <li class="empty">{overdueCount ? 'Nothing else out.' : 'Everything is home. 👌'}</li>
          {/each}
        </ul>
      </section>

      <section class="panel" aria-labelledby="aw-h">
        <h2 id="aw-h">Awaiting inspection <span class="count">{home.awaiting.length}</span></h2>
        <ul>
          {#each home.awaiting as a (a.kind + a.toolId)}
            <li>
              <button class="row-btn" onclick={() => inspect(a)}>
                <Photo id={a.toolPhotoId} size={32} zoom={false} />
                <span class="what grow">
                  <strong>{a.toolName}</strong> <span class="mono faint">{a.assetTag}</span>
                  <span class="small muted block">
                    {a.kind === 'dropoff' ? `Dropped off by ${a.borrowerName}` : `Problem reported by ${a.borrowerName ?? 'someone'}`}
                    · {when(a.since, app.nowMs)}{a.note ? ` · “${a.note}”` : ''}
                  </span>
                </span>
                {#if a.kind === 'problem_report'}<span class="badge badge-warning">Problem</span>{/if}
                {#if a.returnedLate}<span class="badge badge-danger">Late</span>{/if}
                <Icon name="chevronRight" />
              </button>
            </li>
          {:else}
            <li class="empty">Nothing waiting.</li>
          {/each}
        </ul>
      </section>
    </div>

    <p class="foot faint">
      {home.availableTools} tools available
      {#if home.openIncidents}· <button class="link" onclick={() => app.withTech(async () => (app.screen = 'incidents'))}>{home.openIncidents} open incident{home.openIncidents === 1 ? '' : 's'}</button>{/if}
    </p>
  {/if}
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    max-width: 1400px;
  }
  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }
  .big {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.9rem;
    min-height: 112px;
    border-radius: 20px;
    border: 0;
    font: inherit;
    font-size: 1.8rem;
    font-weight: 750;
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition: transform 0.06s;
  }
  .big:active {
    transform: scale(0.99);
  }
  .checkout {
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .return {
    background: var(--surface);
    color: var(--text);
    border: 2px solid var(--border-strong);
  }
  .return:hover {
    background: var(--surface-2);
  }
  .search {
    position: relative;
  }
  .search > :global(.icon) {
    position: absolute;
    left: 16px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-faint);
  }
  .search .input {
    min-height: 58px;
    padding-left: 50px;
    font-size: 1.1rem;
    border-radius: var(--radius-lg);
  }
  .reading {
    position: absolute;
    right: 14px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.85rem;
    padding: 0.2rem 0.5rem;
    background: var(--accent-soft);
    border-radius: 6px;
  }
  .hits {
    position: absolute;
    z-index: 10;
    left: 0;
    right: 0;
    top: calc(100% + 4px);
    list-style: none;
    margin: 0;
    padding: 0.3rem;
    box-shadow: var(--shadow-lg);
  }
  .hits button {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    min-height: 48px;
    padding: 0.3rem 0.6rem;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .hits button:hover {
    background: var(--surface-2);
  }
  .eod {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    padding: 0.75rem 1rem;
    border-radius: var(--radius-lg);
    background: var(--warning-soft);
    border: 1px solid var(--warning);
  }
  .panels {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  .panel h2 {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.8rem 1rem;
    font-size: 1rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }
  .count {
    margin-left: auto;
    min-width: 28px;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: var(--surface-3);
    text-align: center;
    font-size: 0.9rem;
  }
  .panel ul {
    list-style: none;
    margin: 0;
    padding: 0.3rem;
    max-height: 420px;
    overflow: auto;
  }
  .panel.overdue {
    border: 2px solid var(--danger);
  }
  .panel.overdue h2 {
    background: var(--danger);
    color: #fff;
    border-bottom: 0;
  }
  :global(:root[data-theme='dark']) .panel.overdue h2 {
    color: #1a0b0b;
  }
  .panel.overdue .count {
    background: rgb(255 255 255 / 0.25);
  }
  .row-btn {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    min-height: 56px;
    padding: 0.4rem 0.6rem;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .row-btn:hover {
    background: var(--surface-2);
  }
  .who {
    display: flex;
    flex-direction: column;
    min-width: 140px;
  }
  .late {
    color: var(--danger);
    font-weight: 700;
    white-space: nowrap;
  }
  .due {
    white-space: nowrap;
    color: var(--text-muted);
  }
  .small {
    font-size: 0.85rem;
  }
  .block {
    display: block;
  }
  .empty {
    padding: 1.2rem;
    text-align: center;
    color: var(--text-faint);
  }
  .foot {
    font-size: 0.9rem;
  }
  .link {
    border: 0;
    background: none;
    padding: 0;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
    text-decoration: underline;
  }
  @media (max-width: 1000px) {
    .panels {
      grid-template-columns: 1fr;
    }
  }
</style>
