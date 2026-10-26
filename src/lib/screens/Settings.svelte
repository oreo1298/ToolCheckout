<script lang="ts">
  import { app } from '../app.svelte';
  import TechGate from '../components/TechGate.svelte';
  import AuditPanel from '../settings/AuditPanel.svelte';
  import DataPanel from '../settings/DataPanel.svelte';
  import GeneralPanel from '../settings/GeneralPanel.svelte';
  import TechniciansPanel from '../settings/TechniciansPanel.svelte';
</script>

<TechGate title="Settings">
  <div class="page">
    <h1>Settings</h1>
    <!-- In the demo, the way back out comes first. -->
    {#if app.status?.demo}
      <DataPanel />
      <GeneralPanel />
    {:else}
      <GeneralPanel />
      <DataPanel />
    {/if}
    <TechniciansPanel />
    <AuditPanel />
  </div>
</TechGate>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    max-width: 1100px;
  }
</style>
