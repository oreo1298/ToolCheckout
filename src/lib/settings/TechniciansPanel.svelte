<script lang="ts">
  import { api, type Technician } from '../api';
  import { app } from '../app.svelte';
  import Avatar from '../components/Avatar.svelte';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import Icon from '../components/Icon.svelte';
  import Modal from '../components/Modal.svelte';
  import PinCreate from '../components/PinCreate.svelte';
  import RecoveryCode from '../components/RecoveryCode.svelte';

  let techs = $state<Technician[]>([]);
  let editing = $state<{ id: number | null; name: string; active: boolean } | null>(null);
  let pinFor = $state<Technician | null>(null);
  let addStep = $state<'name' | 'pin'>('name');
  let newCode = $state<string | null>(null);
  let askRegenerate = $state(false);

  $effect(() => {
    void app.dataVersion;
    api.listTechnicians().then((t) => (techs = t), app.error);
  });

  async function saveEdit(e: SubmitEvent) {
    e.preventDefault();
    if (!editing) return;
    if (editing.id === null) {
      addStep = 'pin';
      return;
    }
    try {
      await app.withTech(() => api.updateTechnician(editing!.id!, editing!.name, editing!.active));
      editing = null;
      app.toast('Technician saved');
    } catch (err) {
      app.error(err);
    }
  }

  async function createWithPin(pin: string) {
    const t = await app.withTech(() => api.addTechnician(editing!.name, pin));
    if (t) {
      app.toast(`Added ${t.name}`);
      editing = null;
    }
  }

  async function resetPin(pin: string) {
    await app.withTech(() => api.setTechnicianPin(pinFor!.id, pin));
    app.toast(`PIN changed for ${pinFor!.name}`);
    pinFor = null;
  }

  async function regenerate() {
    askRegenerate = false;
    try {
      const code = await app.withTech(() => api.regenerateRecoveryCode());
      if (code) newCode = code;
    } catch (e) {
      app.error(e);
    }
  }
</script>

<section class="card card-pad stack">
  <div class="row">
    <h2 class="grow">Technicians</h2>
    <button class="btn" onclick={() => ((editing = { id: null, name: '', active: true }), (addStep = 'name'))}>
      <Icon name="plus" /> Add technician
    </button>
  </div>
  <table class="table">
    <tbody>
      {#each techs as t (t.id)}
        <tr>
          <td style="width:48px"><Avatar name={t.name} size={34} muted={!t.active} /></td>
          <td>
            <strong>{t.name}</strong>
            {#if !t.active}<span class="badge badge-neutral">Inactive</span>{/if}
            {#if app.session?.techId === t.id}<span class="badge badge-accent">Unlocked now</span>{/if}
          </td>
          <td class="num">
            <button class="btn btn-sm" onclick={() => (pinFor = t)}><Icon name="key" size={16} /> Change PIN</button>
            <button class="btn btn-sm" onclick={() => (editing = { id: t.id, name: t.name, active: t.active })}>
              <Icon name="edit" size={16} /> Edit
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  <div class="row wrap">
    <p class="muted grow">
      The recovery code resets a forgotten technician PIN. Make a new one if it may have been seen by someone else.
    </p>
    <button class="btn" onclick={() => (askRegenerate = true)}><Icon name="refresh" /> New recovery code</button>
  </div>
</section>

{#if editing}
  <Modal title={editing.id === null ? 'Add technician' : 'Edit technician'} size="sm" onclose={() => (editing = null)}>
    {#if addStep === 'pin' && editing.id === null}
      <PinCreate min={4} max={8} label="Choose a PIN for {editing.name}" hint="6 digits recommended" onchosen={createWithPin} />
    {:else}
      <form class="stack" onsubmit={saveEdit}>
        <div class="field">
          <label for="tn">Name</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input id="tn" class="input" bind:value={editing.name} autofocus required />
        </div>
        {#if editing.id !== null}
          <label class="check"><input type="checkbox" bind:checked={editing.active} /> Active</label>
        {/if}
        <button class="btn btn-primary" disabled={!editing.name.trim()}>
          {editing.id === null ? 'Next: choose PIN' : 'Save'}
        </button>
      </form>
    {/if}
  </Modal>
{/if}

{#if pinFor}
  <Modal title="Change PIN — {pinFor.name}" size="sm" onclose={() => (pinFor = null)}>
    <PinCreate min={4} max={8} label="New PIN" hint="6 digits recommended" onchosen={resetPin} />
  </Modal>
{/if}

{#if askRegenerate}
  <ConfirmDialog title="Make a new recovery code?" confirmLabel="Make new code" onconfirm={regenerate} onclose={() => (askRegenerate = false)}>
    <p>The old code stops working immediately. Write the new one down and keep it somewhere safe.</p>
  </ConfirmDialog>
{/if}

{#if newCode}
  <Modal title="New recovery code" size="sm" onclose={() => {}} dismissible={false}>
    <RecoveryCode code={newCode} ondone={() => (newCode = null)} doneLabel="Done" />
  </Modal>
{/if}
