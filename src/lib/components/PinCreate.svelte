<script lang="ts">
  // Choose a new PIN: enter it, then enter it again.
  import PinPad from './PinPad.svelte';

  let {
    min = 4,
    max = 4,
    label = 'Choose a PIN',
    hint,
    onchosen,
  }: {
    min?: number;
    max?: number;
    label?: string;
    hint?: string;
    onchosen: (pin: string) => Promise<void> | void;
  } = $props();

  let first = $state<string | null>(null);
  let key = $state(0);

  async function enter(pin: string) {
    if (first === null) {
      first = pin;
      key++;
      return;
    }
    if (pin !== first) {
      first = null;
      key++;
      throw { code: 'mismatch', message: "PINs didn't match — start again" };
    }
    await onchosen(pin);
  }
</script>

<div class="stack" style="align-items:center">
  {#if hint}<p class="muted" style="text-align:center">{hint}</p>{/if}
  {#key key}
    <PinPad {min} {max} label={first === null ? label : 'Enter it again'} submitLabel="Next" onsubmit={enter} />
  {/key}
</div>
