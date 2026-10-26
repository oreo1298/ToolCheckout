<script lang="ts">
  // Shows a recovery code once, and won't continue until the person confirms
  // they wrote it down.
  let {
    code,
    ondone,
    doneLabel = 'Continue',
  }: { code: string; ondone: () => void; doneLabel?: string } = $props();
  let saved = $state(false);
</script>

<div class="stack">
  <div class="code mono" aria-label="Recovery code">{code}</div>
  <p class="muted">
    Write this down and keep it somewhere safe, away from the counter. It is shown <strong>only now</strong>. With it, anyone
    can reset a technician PIN; without it, a forgotten PIN can't be recovered.
  </p>
  <label class="check">
    <input type="checkbox" bind:checked={saved} />
    I wrote the recovery code down
  </label>
  <button class="btn btn-primary btn-lg" disabled={!saved} onclick={ondone}>{doneLabel}</button>
</div>

<style>
  .code {
    font-size: 1.6rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-align: center;
    padding: 1rem;
    border: 2px dashed var(--accent);
    border-radius: var(--radius-lg);
    background: var(--accent-soft);
    user-select: all;
  }
</style>
