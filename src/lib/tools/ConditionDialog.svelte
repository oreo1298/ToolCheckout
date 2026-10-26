<script lang="ts">
  // Set a tool's condition: manual update (reason required), back from
  // repair, or reinstating a retired tool.
  import { api, type Condition, type Tool } from '../api';
  import { app } from '../app.svelte';
  import ConditionPicker from '../components/ConditionPicker.svelte';
  import Modal from '../components/Modal.svelte';
  import PhotoInput from '../components/PhotoInput.svelte';
  import { needsRepair } from '../domain';

  let {
    tool,
    mode,
    onclose,
  }: { tool: Tool; mode: 'update' | 'repair' | 'reinstate'; onclose: () => void } = $props();

  // A manual update starts from the current condition; the others start blank.
  function initial(): { condition: Condition | null; notes: string } {
    return mode === 'update' ? { condition: tool.condition, notes: tool.conditionNotes } : { condition: null, notes: '' };
  }
  const init = initial();
  let condition = $state<Condition | null>(init.condition);
  let notes = $state(init.notes);
  let photoId = $state<number | null>(null);
  let reason = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);

  const title = $derived(
    mode === 'update' ? `Update condition — ${tool.assetTag}` : mode === 'repair' ? `Back from repair — ${tool.assetTag}` : `Reinstate ${tool.assetTag}`,
  );

  async function save(e: SubmitEvent) {
    e.preventDefault();
    error = null;
    if (!condition) return void (error = 'Choose a condition');
    if (mode === 'update' && !reason.trim()) return void (error = 'Please give a reason for the change');
    if (mode !== 'update' && needsRepair(condition)) {
      return void (error = 'A damaged or broken tool can’t go back on the shelf. Keep it out for repair or retire it.');
    }
    busy = true;
    try {
      const c = condition;
      const done = await app.withTech(() =>
        mode === 'update'
          ? api.updateToolCondition(tool.id, c, notes, photoId, reason)
          : mode === 'repair'
            ? api.toolBackFromRepair(tool.id, c, notes, photoId)
            : api.reinstateTool(tool.id, c, notes),
      );
      if (done) {
        app.toast(mode === 'update' ? 'Condition updated' : `${tool.name} is available again`);
        onclose();
      }
    } catch (err) {
      error = (err as { message: string }).message;
    } finally {
      busy = false;
    }
  }
</script>

<Modal {title} size="lg" {onclose}>
  <form id="condform" class="stack" onsubmit={save}>
    <ConditionPicker bind:value={condition} compareTo={mode === 'update' ? tool.condition : null} compareLabel="current" />
    <div class="field">
      <label for="cd-notes">Condition notes</label>
      <input id="cd-notes" class="input" bind:value={notes} placeholder="What does it look like? e.g. tip slightly bent" />
    </div>
    {#if mode !== 'reinstate'}
      <PhotoInput bind:value={photoId} />
    {/if}
    {#if mode === 'update'}
      <div class="field">
        <label for="cd-reason">Reason for the change (required)</label>
        <input id="cd-reason" class="input" bind:value={reason} placeholder="e.g. Monthly check, dropped on the floor" />
      </div>
    {/if}
    {#if error}<p class="error-text" role="alert">{error}</p>{/if}
  </form>
  {#snippet footer()}
    <button type="button" class="btn" onclick={onclose}>Cancel</button>
    <button form="condform" class="btn btn-primary" disabled={busy}>Save</button>
  {/snippet}
</Modal>
