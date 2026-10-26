<script lang="ts">
  import type { Condition } from '../api';
  import { conditionInfo } from '../domain';
  let { condition }: { condition: Condition } = $props();
  const info = $derived(conditionInfo(condition));
</script>

<span class="badge badge-{info.tone}" title="Condition: {info.hint}"><span class="dot"></span>{info.label}</span>

<style>
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }
</style>
