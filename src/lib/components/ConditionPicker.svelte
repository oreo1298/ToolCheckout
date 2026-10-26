<script lang="ts">
  // Big, touch-friendly condition choice. Arrow keys move between options.
  import type { Condition } from '../api';
  import { CONDITIONS, conditionRank } from '../domain';

  let {
    value = $bindable(),
    compareTo,
    compareLabel = 'at checkout',
    label = 'Condition',
  }: { value: Condition | null; compareTo?: Condition | null; compareLabel?: string; label?: string } = $props();

  function key(e: KeyboardEvent, i: number) {
    const next = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? i + 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? i - 1 : -1;
    if (next >= 0 && next < CONDITIONS.length) {
      value = CONDITIONS[next].value;
      ((e.currentTarget as HTMLElement).parentElement?.children[next] as HTMLElement)?.focus();
      e.preventDefault();
    }
  }
</script>

<div class="field">
  <span class="label" id="cp-label">{label}</span>
  <div class="picker" role="radiogroup" aria-labelledby="cp-label">
    {#each CONDITIONS as c, i (c.value)}
      {@const worse = compareTo != null && conditionRank(c.value) > conditionRank(compareTo)}
      <button
        type="button"
        role="radio"
        aria-checked={value === c.value}
        tabindex={value === c.value || (value == null && i === 0) ? 0 : -1}
        class="opt tone-{c.tone}"
        class:on={value === c.value}
        onclick={() => (value = c.value)}
        onkeydown={(e) => key(e, i)}
        title={c.hint}
      >
        <span class="name">{c.label}</span>
        {#if compareTo === c.value}<span class="was">{compareLabel}</span>{:else if worse && value === c.value}<span class="was">worse</span>{/if}
      </button>
    {/each}
  </div>
  {#if value}<span class="faint hint">{CONDITIONS.find((c) => c.value === value)?.hint}</span>{/if}
</div>

<style>
  .picker {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 0.4rem;
  }
  .opt {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.1rem;
    min-height: 56px;
    padding: 0.3rem;
    border-radius: var(--radius);
    border: 2px solid var(--border);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-weight: 650;
    cursor: pointer;
  }
  .opt:hover {
    background: var(--surface-2);
  }
  .opt.on.tone-success {
    border-color: var(--success);
    background: var(--success-soft);
  }
  .opt.on.tone-info {
    border-color: var(--info);
    background: var(--info-soft);
  }
  .opt.on.tone-warning {
    border-color: var(--warning);
    background: var(--warning-soft);
  }
  .opt.on.tone-danger {
    border-color: var(--danger);
    background: var(--danger-soft);
  }
  .was {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--text-muted);
  }
  .hint {
    font-size: 0.85rem;
  }
  @media (max-width: 700px) {
    .picker {
      grid-template-columns: repeat(3, 1fr);
    }
  }
</style>
