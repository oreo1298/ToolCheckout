<script lang="ts">
  // Pick an image, downscale it, store it; binds the stored photo id.
  import { api } from '../api';
  import { app } from '../app.svelte';
  import { fileToJpeg } from '../photo';
  import Icon from './Icon.svelte';
  import Photo from './Photo.svelte';

  let { value = $bindable(), label = 'Photo (optional)' }: { value: number | null; label?: string } = $props();
  let busy = $state(false);
  let input: HTMLInputElement;

  async function picked() {
    const file = input.files?.[0];
    if (!file) return;
    busy = true;
    try {
      value = await api.savePhoto(await fileToJpeg(file));
    } catch (e) {
      app.error(e);
    } finally {
      busy = false;
      input.value = '';
    }
  }
</script>

<div class="field">
  <span class="label">{label}</span>
  <div class="row">
    <Photo id={value} size={64} />
    <button type="button" class="btn btn-sm" onclick={() => input.click()} disabled={busy}>
      <Icon name="camera" size={16} />
      {busy ? 'Saving…' : value ? 'Replace photo' : 'Add photo'}
    </button>
    {#if value}
      <button type="button" class="btn btn-sm btn-ghost" onclick={() => (value = null)}>Remove</button>
    {/if}
    <input bind:this={input} type="file" accept="image/*" hidden onchange={picked} />
  </div>
</div>
