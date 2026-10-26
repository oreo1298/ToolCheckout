<script lang="ts">
  // Technician's return inspection for one tool: condition (compared with
  // what it was when lent), notes, photo. Warns before a damage incident.
  import type { Condition } from '../api';
  import ConditionBadge from '../components/ConditionBadge.svelte';
  import ConditionPicker from '../components/ConditionPicker.svelte';
  import PhotoInput from '../components/PhotoInput.svelte';
  import { conditionInfo, isDowngrade, needsRepair } from '../domain';

  let {
    before,
    beforeLabel = 'at checkout',
    borrowerName,
    condition = $bindable(),
    notes = $bindable(),
    photoId = $bindable(),
  }: {
    before: Condition;
    beforeLabel?: string;
    borrowerName: string | null;
    condition: Condition | null;
    notes: string;
    photoId: number | null;
  } = $props();

  const worse = $derived(condition != null && isDowngrade(condition, before));
</script>

<div class="stack">
  <div class="row muted">Condition {beforeLabel}: <ConditionBadge condition={before} /></div>
  <ConditionPicker bind:value={condition} compareTo={before} compareLabel={beforeLabel} label="Condition now" />
  {#if worse && condition}
    <div class="warn" role="status">
      Worse than {beforeLabel} ({conditionInfo(before).label} → {conditionInfo(condition).label}).
      {#if borrowerName}A damage incident will be opened for <strong>{borrowerName}</strong>.{:else}A damage incident will be opened.{/if}
      Add a note and, ideally, a photo.
    </div>
  {/if}
  {#if condition && needsRepair(condition)}
    <div class="info" role="status">It will be marked <strong>Out for repair</strong> instead of going back on the shelf.</div>
  {/if}
  <div class="field">
    <label for="if-notes">Notes{worse ? ' (what happened?)' : ''}</label>
    <input id="if-notes" class="input" bind:value={notes} placeholder={worse ? 'e.g. tip bent, case cracked' : 'optional'} />
  </div>
  <PhotoInput bind:value={photoId} />
</div>

<style>
  .warn,
  .info {
    padding: 0.7rem 0.9rem;
    border-radius: var(--radius);
  }
  .warn {
    background: var(--danger-soft);
    border: 1px solid var(--danger-border);
  }
  .info {
    background: var(--info-soft);
  }
</style>
