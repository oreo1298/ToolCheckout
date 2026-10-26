<script lang="ts">
  import Modal from './Modal.svelte';

  let {
    title,
    label = 'Reason',
    placeholder = '',
    confirm = 'Confirm',
    danger = false,
    message,
    onsubmit,
    onclose,
  }: {
    title: string;
    label?: string;
    placeholder?: string;
    confirm?: string;
    danger?: boolean;
    message?: string;
    onsubmit: (text: string) => Promise<void>;
    onclose: () => void;
  } = $props();

  let text = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!text.trim()) {
      error = 'Please fill this in';
      return;
    }
    busy = true;
    try {
      await onsubmit(text.trim());
    } catch (err) {
      error = (err as { message: string }).message;
    } finally {
      busy = false;
    }
  }
</script>

<Modal {title} size="sm" {onclose}>
  <form id="reasonform" class="stack" onsubmit={submit}>
    {#if message}<p>{message}</p>{/if}
    <div class="field">
      <label for="rd-text">{label}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <textarea id="rd-text" class="input" bind:value={text} {placeholder} rows="3" autofocus></textarea>
    </div>
    {#if error}<p class="error-text">{error}</p>{/if}
  </form>
  {#snippet footer()}
    <button type="button" class="btn" onclick={onclose}>Cancel</button>
    <button form="reasonform" class="btn {danger ? 'btn-danger' : 'btn-primary'}" disabled={busy}>{confirm}</button>
  {/snippet}
</Modal>
