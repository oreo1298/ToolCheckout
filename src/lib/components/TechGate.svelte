<script lang="ts">
  // Renders its content only while a technician is unlocked.
  import type { Snippet } from 'svelte';
  import { app } from '../app.svelte';
  import Icon from './Icon.svelte';

  let { title, children }: { title: string; children: Snippet } = $props();
</script>

{#if app.session}
  {@render children()}
{:else}
  <div class="gate card">
    <span class="lock"><Icon name="lock" size={30} /></span>
    <h2>{title} is for technicians</h2>
    <p class="muted">Enter a technician PIN to continue. Tech mode locks again after inactivity.</p>
    <button class="btn btn-primary btn-lg" onclick={() => app.requestUnlock()}>
      <Icon name="unlock" /> Unlock
    </button>
  </div>
{/if}

<style>
  .gate {
    max-width: 460px;
    margin: 3rem auto;
    padding: 2rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    text-align: center;
  }
  .lock {
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: var(--surface-3);
    color: var(--text-muted);
  }
</style>
