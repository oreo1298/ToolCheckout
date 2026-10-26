<script lang="ts">
  import { photoUrl } from '../photo';
  import Icon from './Icon.svelte';

  let { id, size = 64, alt = 'Photo', zoom = true }: { id: number | null; size?: number; alt?: string; zoom?: boolean } =
    $props();
  let src = $state<string | null>(null);
  let big = $state(false);

  $effect(() => {
    src = null;
    if (id != null) photoUrl(id).then((u) => (src = u), () => (src = null));
  });
</script>

{#if id == null}
  <span class="ph" style="width:{size}px;height:{size}px" aria-hidden="true"><Icon name="camera" size={Math.round(size / 2.6)} /></span>
{:else if src}
  <button type="button" class="thumb" style="width:{size}px;height:{size}px" onclick={() => zoom && (big = true)} aria-label="View {alt}">
    <img {src} {alt} />
  </button>
{:else}
  <span class="ph" style="width:{size}px;height:{size}px"></span>
{/if}

{#if big && src}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="lightbox" onclick={() => (big = false)}>
    <img {src} {alt} />
  </div>
{/if}

<style>
  .ph,
  .thumb {
    flex: none;
    display: inline-grid;
    place-items: center;
    border-radius: var(--radius);
    background: var(--surface-3);
    color: var(--text-faint);
    overflow: hidden;
    border: 1px solid var(--border);
    padding: 0;
  }
  .thumb {
    cursor: zoom-in;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 2000;
    background: rgb(0 0 0 / 0.85);
    display: grid;
    place-items: center;
    cursor: zoom-out;
  }
  .lightbox img {
    max-width: 92vw;
    max-height: 92vh;
    border-radius: var(--radius);
  }
</style>
