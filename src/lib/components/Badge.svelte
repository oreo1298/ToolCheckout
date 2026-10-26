<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Tone } from '../domain';
  let { tone = 'neutral', children }: { tone?: Tone; children: Snippet } = $props();
</script>

<span class="badge badge-{tone}">{@render children()}</span>
