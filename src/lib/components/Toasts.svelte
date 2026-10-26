<script lang="ts">
  import { app } from '../app.svelte';
  import Icon from './Icon.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}" role={t.kind === 'error' ? 'alert' : 'status'}>
      <Icon name={t.kind === 'error' ? 'alert' : t.kind === 'info' ? 'info' : 'check'} />
      <span class="grow">{t.text}</span>
      <button class="btn btn-ghost btn-sm" onclick={() => app.dismissToast(t.id)} aria-label="Dismiss">
        <Icon name="x" size={16} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    z-index: 1000;
    max-width: min(440px, calc(100vw - 32px));
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.5rem 0.5rem 0.9rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
    font-weight: 550;
  }
  .toast.success {
    border-left: 4px solid var(--success);
  }
  .toast.success :global(.icon) {
    color: var(--success);
  }
  .toast.error {
    border-left: 4px solid var(--danger);
  }
  .toast.error :global(.icon) {
    color: var(--danger);
  }
  .toast.info {
    border-left: 4px solid var(--info);
  }
</style>
