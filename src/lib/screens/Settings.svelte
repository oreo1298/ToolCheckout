<script lang="ts">
  import TechGate from '../components/TechGate.svelte';
  import AuditPanel from '../settings/AuditPanel.svelte';
  import GeneralPanel from '../settings/GeneralPanel.svelte';
  import TechniciansPanel from '../settings/TechniciansPanel.svelte';
</script>

<TechGate title="Settings">
  <div class="page">
    <h1>Settings</h1>
    <GeneralPanel />
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
