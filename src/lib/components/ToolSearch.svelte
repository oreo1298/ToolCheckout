<script lang="ts">
  // Search box + results for picking a tool. Typing or scanning a tag and
  // pressing Enter picks the exact match immediately.
  import { api, type ToolRow, type ToolStatus } from '../api';
  import { STATUSES } from '../domain';
  import Icon from './Icon.svelte';
  import Photo from './Photo.svelte';

  let {
    onpick,
    want,
    exclude = [],
    placeholder = 'Search or scan a tool…',
    autofocus = false,
  }: {
    onpick: (t: ToolRow) => void;
    /** Statuses that can be picked; others show greyed with their status. */
    want: ToolStatus[];
    exclude?: number[];
    placeholder?: string;
    autofocus?: boolean;
  } = $props();

  let q = $state('');
  let results = $state<ToolRow[]>([]);
  let input: HTMLInputElement;

  $effect(() => {
    if (autofocus) input?.focus();
  });

  $effect(() => {
    const query = q.trim();
    if (!query) {
      results = [];
      return;
    }
    const t = setTimeout(() => {
      api.listTools({ search: query, limit: 12 }).then((r) => (results = r.filter((x) => !exclude.includes(x.id))));
    }, 100);
    return () => clearTimeout(t);
  });

  function pick(t: ToolRow) {
    if (!want.includes(t.status)) return;
    onpick(t);
    q = '';
    results = [];
  }

  async function enter(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    const code = q.trim();
    if (!code) return;
    const r = await api.resolveScan(code);
    if (r.kind === 'tool') {
      if (want.includes(r.tool.status)) pick(r.tool);
      else results = [r.tool];
    } else if (results.length === 1) {
      pick(results[0]);
    }
  }
</script>

<div class="ts">
  <div class="box">
    <Icon name="search" />
    <input
      bind:this={input}
      class="input"
      type="search"
      bind:value={q}
      onkeydown={enter}
      {placeholder}
      aria-label={placeholder}
      autocomplete="off"
    />
  </div>
  {#if results.length}
    <ul class="results card" role="listbox">
      {#each results as t (t.id)}
        {@const ok = want.includes(t.status)}
        <li>
          <button type="button" class="res" disabled={!ok} onclick={() => pick(t)} role="option" aria-selected="false">
            <Photo id={t.photoId} size={36} zoom={false} />
            <span class="grow">
              <strong>{t.name}</strong>
              <span class="mono faint">{t.assetTag}</span>
              {#if t.techOnly}<Icon name="lock" size={13} label="Tech-only" />{/if}
            </span>
            {#if !ok}
              <span class="badge badge-{STATUSES[t.status].tone}">
                {STATUSES[t.status].label}{t.borrowerName ? ` · ${t.borrowerName}` : ''}
              </span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .ts {
    position: relative;
  }
  .box {
    position: relative;
  }
  .box :global(.icon) {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-faint);
  }
  .box .input {
    padding-left: 40px;
  }
  .results {
    position: absolute;
    z-index: 20;
    left: 0;
    right: 0;
    top: calc(100% + 4px);
    list-style: none;
    margin: 0;
    padding: 0.25rem;
    max-height: 340px;
    overflow: auto;
    box-shadow: var(--shadow-lg);
  }
  .res {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    min-height: 52px;
    padding: 0.35rem 0.6rem;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .res:hover:not(:disabled) {
    background: var(--surface-2);
  }
  .res:disabled {
    cursor: default;
    opacity: 0.75;
  }
  .mono {
    margin-left: 0.4rem;
    font-size: 0.85rem;
  }
</style>
