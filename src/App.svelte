<script lang="ts">
  import Icon from './lib/components/Icon.svelte';

  const nav = [
    { id: 'home', label: 'Home', icon: 'home' },
    { id: 'tools', label: 'Tools', icon: 'tool' },
    { id: 'borrowers', label: 'Borrowers', icon: 'users' },
    { id: 'incidents', label: 'Incidents', icon: 'alert' },
    { id: 'reports', label: 'Reports', icon: 'chart' },
    { id: 'settings', label: 'Settings', icon: 'settings' },
  ];
  let screen = $state('home');
</script>

<div class="shell">
  <header class="topbar">
    <div class="brand">
      <span class="logo" aria-hidden="true"><Icon name="tool" size={18} /></span>
      <span>ToolCheckout</span>
    </div>
  </header>
  <nav class="rail" aria-label="Main">
    {#each nav as item (item.id)}
      <button
        class="rail-item"
        class:active={screen === item.id}
        aria-current={screen === item.id ? 'page' : undefined}
        onclick={() => (screen = item.id)}
      >
        <Icon name={item.icon} size={22} />
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>
  <main class="content">
    <h1>{nav.find((n) => n.id === screen)?.label}</h1>
    <p class="muted">Coming up in the next milestones.</p>
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 96px 1fr;
    grid-template-rows: 56px 1fr;
    height: 100%;
  }
  .topbar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0 1rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-weight: 700;
    font-size: 1.05rem;
  }
  .logo {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .rail {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.5rem;
    background: var(--surface);
    border-right: 1px solid var(--border);
  }
  .rail-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    min-height: 64px;
    justify-content: center;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
  }
  .rail-item:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .rail-item.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .content {
    overflow: auto;
    padding: 1.25rem 1.5rem;
  }
</style>
