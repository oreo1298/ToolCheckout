<script lang="ts">
  // Touch keypad + keyboard entry for PINs. Fixed-length PINs submit
  // automatically on the last digit; variable-length ones on Enter/OK.
  // onsubmit throws an ApiError for wrong PINs / lockouts, shown here.
  import { asApiError } from '../api';
  import { countdown } from '../format';
  import Icon from './Icon.svelte';

  let {
    min = 4,
    max = 4,
    label = 'Enter PIN',
    submitLabel = 'OK',
    onsubmit,
    autofocus = true,
  }: {
    min?: number;
    max?: number;
    label?: string;
    submitLabel?: string;
    onsubmit: (pin: string) => Promise<void> | void;
    autofocus?: boolean;
  } = $props();

  let pin = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);
  let lockedUntil = $state<number | null>(null);
  let now = $state(Date.now());
  let shake = $state(false);
  let root: HTMLDivElement;

  const locked = $derived(lockedUntil !== null && lockedUntil > now);
  const fixed = $derived(min === max);

  $effect(() => {
    if (!lockedUntil) return;
    const t = setInterval(() => {
      now = Date.now();
      if (lockedUntil && now >= lockedUntil) {
        lockedUntil = null;
        error = null;
      }
    }, 250);
    return () => clearInterval(t);
  });

  $effect(() => {
    if (autofocus) root?.focus();
  });

  function press(d: string) {
    if (busy || locked || pin.length >= max) return;
    error = null;
    pin += d;
    if (fixed && pin.length === max) void submit();
  }

  function back() {
    if (!busy) pin = pin.slice(0, -1);
  }

  async function submit() {
    if (busy || locked || pin.length < min) return;
    busy = true;
    try {
      await onsubmit(pin);
    } catch (e) {
      const err = asApiError(e);
      if (err.code === 'locked_out' && err.lockedUntil) {
        lockedUntil = err.lockedUntil;
        now = Date.now();
        error = 'Too many wrong PINs.';
      } else if (err.code === 'wrong_pin') {
        error =
          err.attemptsLeft && err.attemptsLeft <= 2
            ? `Wrong PIN — ${err.attemptsLeft} ${err.attemptsLeft === 1 ? 'try' : 'tries'} left before lockout`
            : 'Wrong PIN';
      } else {
        error = err.message;
      }
      shake = true;
      setTimeout(() => (shake = false), 400);
    } finally {
      pin = '';
      busy = false;
    }
  }

  function keydown(e: KeyboardEvent) {
    if (e.key >= '0' && e.key <= '9') {
      press(e.key);
      e.preventDefault();
    } else if (e.key === 'Backspace') {
      back();
      e.preventDefault();
    } else if (e.key === 'Enter') {
      void submit();
      e.preventDefault();
    }
  }
</script>

<!-- Focusable group so keyboard digits work without a text field (no on-screen keyboard pops up). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div class="pinpad" bind:this={root} tabindex="0" onkeydown={keydown} role="group" aria-label={label}>
  <div class="label">{label}</div>
  <div class="dots" class:shake aria-live="polite" aria-label="{pin.length} digits entered">
    {#each Array(fixed ? max : Math.max(min, pin.length)) as _, i (i)}
      <span class="dot" class:filled={i < pin.length}></span>
    {/each}
  </div>
  <div class="msg" role="alert">
    {#if locked && lockedUntil}
      <span class="error-text">{error} Try again in {countdown(lockedUntil - now)}</span>
    {:else if error}
      <span class="error-text">{error}</span>
    {:else if !fixed}
      <span class="faint">{min}–{max} digits, then OK</span>
    {/if}
  </div>
  <div class="keys">
    {#each ['1', '2', '3', '4', '5', '6', '7', '8', '9'] as d (d)}
      <button type="button" class="key" onclick={() => press(d)} disabled={busy || locked} tabindex="-1">{d}</button>
    {/each}
    <button type="button" class="key fn" onclick={back} disabled={busy || locked || !pin} aria-label="Delete digit" tabindex="-1">
      <Icon name="chevronLeft" size={22} />
    </button>
    <button type="button" class="key" onclick={() => press('0')} disabled={busy || locked} tabindex="-1">0</button>
    {#if fixed}
      <button type="button" class="key fn" onclick={() => (pin = '')} disabled={busy || locked || !pin} tabindex="-1">Clear</button>
    {:else}
      <button
        type="button"
        class="key ok"
        onclick={submit}
        disabled={busy || locked || pin.length < min}
        tabindex="-1">{submitLabel}</button
      >
    {/if}
  </div>
</div>

<style>
  .pinpad {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    outline: none;
    padding: 0.25rem;
    border-radius: var(--radius-lg);
  }
  .pinpad:focus-visible {
    outline: 2px dashed var(--border-strong);
  }
  .label {
    font-weight: 650;
    font-size: 1.05rem;
  }
  .dots {
    display: flex;
    gap: 0.7rem;
    min-height: 22px;
    align-items: center;
  }
  .dot {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
  }
  .dot.filled {
    background: var(--accent);
    border-color: var(--accent);
  }
  .shake {
    animation: shake 0.35s;
  }
  @keyframes shake {
    25% {
      transform: translateX(-8px);
    }
    50% {
      transform: translateX(8px);
    }
    75% {
      transform: translateX(-4px);
    }
  }
  .msg {
    min-height: 1.4em;
    font-size: 0.9rem;
    text-align: center;
  }
  .keys {
    display: grid;
    grid-template-columns: repeat(3, 76px);
    gap: 0.6rem;
  }
  .key {
    height: 64px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: 1.5rem;
    font-weight: 600;
    cursor: pointer;
    display: grid;
    place-items: center;
  }
  .key:hover:not(:disabled) {
    background: var(--surface-3);
  }
  .key:active:not(:disabled) {
    transform: scale(0.97);
  }
  .key:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .key.fn {
    font-size: 0.95rem;
    color: var(--text-muted);
  }
  .key.ok {
    font-size: 1.05rem;
    background: var(--accent);
    color: var(--accent-contrast);
    border-color: var(--accent);
  }
</style>
