<script lang="ts">
  // Technician inspects one item from "Awaiting inspection": a drop-off or a
  // tool a borrower flagged at checkout.
  import { api, asApiError, type AwaitingItem, type Condition } from '../api';
  import { app } from '../app.svelte';
  import Avatar from '../components/Avatar.svelte';
  import Modal from '../components/Modal.svelte';
  import Photo from '../components/Photo.svelte';
  import { when } from '../format';
  import InspectForm from './InspectForm.svelte';

  let { item, onclose }: { item: AwaitingItem; onclose: () => void } = $props();

  let condition = $state<Condition | null>(null);
  let notes = $state('');
  let photoId = $state<number | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  const isDrop = $derived(item.kind === 'dropoff');

  async function submit() {
    error = null;
    if (!condition) return void (error = 'Choose a condition');
    busy = true;
    try {
      const c = condition;
      const r = await app.withTech(
        () =>
          isDrop && item.checkoutId
            ? api.inspectReturn(item.checkoutId, c, notes, photoId)
            : api.inspectTool(item.toolId, c, notes, photoId),
        'Inspection needs a technician PIN.',
      );
      if (r) {
        app.toast(
          `${item.assetTag} inspected — ${r.tool.status === 'out_for_repair' ? 'out for repair' : 'back on the shelf'}` +
            (r.incident ? '. Damage incident opened.' : ''),
          r.incident ? 'info' : 'success',
        );
        onclose();
      }
    } catch (e) {
      error = asApiError(e).message;
    } finally {
      busy = false;
    }
  }
</script>

<Modal title="Inspect {item.assetTag} · {item.toolName}" size="lg" {onclose}>
  <div class="stack">
    <div class="row">
      <Photo id={item.toolPhotoId} size={64} />
      <div class="grow">
        {#if isDrop}
          <div class="row"><Avatar name={item.borrowerName ?? '?'} size={24} /> Dropped off by <strong>{item.borrowerName}</strong> · {when(item.since, app.nowMs)}</div>
          {#if item.returnedLate}<span class="badge badge-danger">Returned late</span>{/if}
        {:else}
          <div>Problem reported by <strong>{item.borrowerName ?? 'someone'}</strong> · {when(item.since, app.nowMs)}</div>
        {/if}
        {#if item.note}<p class="note">“{item.note}”</p>{/if}
      </div>
    </div>
    <InspectForm
      before={item.conditionBefore}
      beforeLabel={isDrop ? 'at checkout' : 'on record'}
      borrowerName={isDrop ? item.borrowerName : null}
      bind:condition
      bind:notes
      bind:photoId
    />
    {#if error}<p class="error-text" role="alert">{error}</p>{/if}
  </div>
  {#snippet footer()}
    <button class="btn" onclick={onclose}>Cancel</button>
    <button class="btn btn-primary btn-lg" disabled={busy} onclick={submit}>Complete inspection</button>
  {/snippet}
</Modal>

<style>
  .note {
    margin-top: 0.3rem;
    padding: 0.5rem 0.7rem;
    border-left: 3px solid var(--warning);
    background: var(--warning-soft);
    border-radius: 4px;
  }
</style>
