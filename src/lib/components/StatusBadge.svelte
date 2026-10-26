<script lang="ts">
  import type { ToolStatus } from '../api';
  import { STATUSES } from '../domain';
  let { status, overdue = false }: { status: ToolStatus; overdue?: boolean } = $props();
  const info = $derived(STATUSES[status]);
</script>

{#if overdue}
  <span class="badge badge-danger">Overdue</span>
{:else}
  <span class="badge badge-{info.tone}">{info.label}</span>
{/if}
