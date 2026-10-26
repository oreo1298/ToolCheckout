<script lang="ts">
  // Technician unlock: pick your name (auto-picked when there's one tech),
  // enter your PIN. "Forgot PIN?" uses the one-time recovery code.
  import { api, type Technician } from '../api';
  import { app } from '../app.svelte';
  import Avatar from '../components/Avatar.svelte';
  import Modal from '../components/Modal.svelte';
  import PinCreate from '../components/PinCreate.svelte';
  import PinPad from '../components/PinPad.svelte';
  import RecoveryCode from '../components/RecoveryCode.svelte';

  let { reason }: { reason?: string } = $props();

  let techs = $state<Technician[]>([]);
  let chosen = $state<Technician | null>(null);
  let mode = $state<'pin' | 'recover' | 'recover-pin' | 'recover-done'>('pin');
  let code = $state('');
  let codeError = $state<string | null>(null);
  let newCode = $state('');

  $effect(() => {
    api.technicianNames().then((t) => {
      techs = t;
      if (t.length === 1) chosen = t[0];
    }, app.error);
  });

  async function submitPin(pin: string) {
    const session = await api.unlock(chosen!.id, pin);
    app.finishUnlock(session);
    app.toast(`Unlocked: ${session.techName}`);
  }

  async function recover(pin: string) {
    try {
      newCode = await api.recover(code, chosen!.id, pin);
      mode = 'recover-done';
    } catch (e) {
      // Back to the code form with the reason (wrong code, locked out, …).
      mode = 'recover';
      codeError = (e as { message: string }).message;
    }
  }
</script>

<Modal title="Technician PIN" size="sm" onclose={() => app.finishUnlock(null)} dismissible={mode !== 'recover-done'}>
  <div class="stack" style="align-items:stretch">
    {#if reason && mode === 'pin'}
      <p class="muted center">{reason}</p>
    {/if}

    {#if !chosen}
      <p class="center">Who's unlocking?</p>
      <div class="techs">
        {#each techs as t (t.id)}
          <button class="btn tech" onclick={() => (chosen = t)}>
            <Avatar name={t.name} size={36} />
            <span>{t.name}</span>
          </button>
        {/each}
      </div>
    {:else if mode === 'pin'}
      <div class="row" style="justify-content:center">
        <Avatar name={chosen.name} size={32} />
        <strong>{chosen.name}</strong>
        {#if techs.length > 1}
          <button class="btn btn-ghost btn-sm" onclick={() => (chosen = null)}>Change</button>
        {/if}
      </div>
      <PinPad min={4} max={8} label="Enter your PIN" submitLabel="Unlock" onsubmit={submitPin} />
      <button class="btn btn-ghost btn-sm" onclick={() => ((mode = 'recover'), (codeError = null))}>Forgot PIN?</button>
    {:else if mode === 'recover'}
      <p>
        Enter the <strong>recovery code</strong> you wrote down when ToolCheckout was set up. It sets a new PIN for
        <strong>{chosen.name}</strong>.
      </p>
      <form
        class="stack"
        onsubmit={(e) => {
          e.preventDefault();
          if (code.replace(/[^0-9a-z]/gi, '').length >= 16) mode = 'recover-pin';
          else codeError = 'Recovery codes have 16 letters and digits';
        }}
      >
        <div class="field">
          <label for="rc">Recovery code</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            id="rc"
            class="input mono"
            autocomplete="off"
            spellcheck="false"
            placeholder="XXXX-XXXX-XXXX-XXXX"
            bind:value={code}
            autofocus
            aria-invalid={!!codeError}
          />
          {#if codeError}<span class="error-text">{codeError}</span>{/if}
        </div>
        <div class="row">
          <button type="button" class="btn" onclick={() => (mode = 'pin')}>Back</button>
          <span class="spacer"></span>
          <button class="btn btn-primary">Next</button>
        </div>
      </form>
    {:else if mode === 'recover-pin'}
      <PinCreate min={4} max={8} label="Choose a new PIN" hint="6 digits recommended" onchosen={recover} />
    {:else}
      <p>PIN changed for <strong>{chosen.name}</strong>. Recovery codes work once, so here is a new one:</p>
      <RecoveryCode code={newCode} ondone={() => ((mode = 'pin'), (code = ''))} doneLabel="Now unlock with the new PIN" />
    {/if}
  </div>
</Modal>

<style>
  .center {
    text-align: center;
  }
  .techs {
    display: grid;
    gap: 0.5rem;
  }
  .tech {
    justify-content: flex-start;
    min-height: 60px;
  }
</style>
