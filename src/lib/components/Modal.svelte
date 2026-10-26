<script lang="ts">
  // Native <dialog>: focus is trapped inside, Esc closes, it sits in the top
  // layer. Mount it with {#if} to open; it calls onclose when dismissed.
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  let {
    title,
    onclose,
    children,
    footer,
    size = 'md',
    dismissible = true,
  }: {
    title: string;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
    size?: 'sm' | 'md' | 'lg' | 'xl';
    dismissible?: boolean;
  } = $props();

  let el: HTMLDialogElement;

  $effect(() => {
    el.showModal();
    return () => el.open && el.close();
  });

  function cancel(e: Event) {
    e.preventDefault();
    if (dismissible) onclose();
  }

  function backdrop(e: MouseEvent) {
    if (dismissible && e.target === el) onclose();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog bind:this={el} class="modal {size}" oncancel={cancel} onclick={backdrop} aria-label={title}>
  <div class="frame">
    <header>
      <h2>{title}</h2>
      {#if dismissible}
        <button class="btn btn-ghost btn-icon" onclick={onclose} aria-label="Close">
          <Icon name="x" />
        </button>
      {/if}
    </header>
    <div class="body">
      {@render children()}
    </div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</dialog>

<style>
  dialog {
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-lg);
    width: min(560px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    overflow: hidden;
  }
  dialog.sm {
    width: min(420px, calc(100vw - 32px));
  }
  dialog.lg {
    width: min(820px, calc(100vw - 32px));
  }
  dialog.xl {
    width: min(1100px, calc(100vw - 32px));
  }
  dialog::backdrop {
    background: var(--scrim);
  }
  .frame {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 48px);
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 0.75rem 0.75rem 1.25rem;
    border-bottom: 1px solid var(--border);
  }
  header h2 {
    flex: 1;
  }
  .body {
    padding: 1.25rem;
    overflow: auto;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 0.75rem 1.25rem;
    border-top: 1px solid var(--border);
    background: var(--surface-2);
  }
</style>
