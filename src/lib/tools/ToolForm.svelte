<script lang="ts">
  // Add, edit or duplicate a tool. Condition is chosen only when adding;
  // later changes go through "Update condition" (with a reason).
  import { api, type Tool, type ToolInput } from '../api';
  import { app } from '../app.svelte';
  import ConditionPicker from '../components/ConditionPicker.svelte';
  import Modal from '../components/Modal.svelte';
  import PhotoInput from '../components/PhotoInput.svelte';
  import { LOAN_PRESETS } from '../domain';
  import { loanPeriod, money, parseMoney } from '../format';

  let {
    mode,
    source,
    onclose,
    onsaved,
  }: {
    mode: 'add' | 'edit' | 'duplicate';
    source?: Tool;
    onclose: () => void;
    onsaved: (t: Tool) => void;
  } = $props();

  function initial(): ToolInput {
    const s = source;
    return {
      name: s?.name ?? '',
      category: s?.category ?? '',
      description: s?.description ?? '',
      serial: mode === 'edit' ? (s?.serial ?? '') : '', // serials are per item
      model: s?.model ?? '',
      photoId: s?.photoId ?? null,
      replacementCostCents: s?.replacementCostCents ?? null,
      homeLocation: s?.homeLocation ?? '',
      defaultLoanMinutes: s?.defaultLoanMinutes ?? app.settings?.defaultLoanMinutes ?? 240,
      techOnly: s?.techOnly ?? false,
      condition: mode === 'edit' ? (s?.condition ?? null) : null,
      conditionNotes: '',
    };
  }

  let form = $state<ToolInput>(initial());
  let cost = $state(form.replacementCostCents != null ? (form.replacementCostCents / 100).toFixed(2) : '');
  let categories = $state<string[]>([]);
  let error = $state<string | null>(null);
  let saving = $state(false);
  const presets = $derived(
    LOAN_PRESETS.some((p) => p.minutes === form.defaultLoanMinutes)
      ? LOAN_PRESETS
      : [...LOAN_PRESETS, { minutes: form.defaultLoanMinutes, label: loanPeriod(form.defaultLoanMinutes) }],
  );

  $effect(() => {
    api.toolCategories().then((c) => (categories = c), () => {});
  });

  async function save(e: SubmitEvent) {
    e.preventDefault();
    error = null;
    const cents = parseMoney(cost);
    if (Number.isNaN(cents)) {
      error = 'Replacement cost should be a number like 24.99';
      return;
    }
    if (mode !== 'edit' && !form.condition) {
      error = "Choose the tool's current condition";
      return;
    }
    saving = true;
    try {
      const input = { ...$state.snapshot(form), replacementCostCents: cents } as ToolInput;
      const saved = await app.withTech(() =>
        mode === 'edit' ? api.updateTool(source!.id, input) : api.createTool(input),
      );
      if (saved) {
        app.toast(mode === 'edit' ? `Saved ${saved.name}` : `Added ${saved.name} as ${saved.assetTag}`);
        onsaved(saved);
      }
    } catch (err) {
      error = (err as { message: string }).message;
    } finally {
      saving = false;
    }
  }

  const title = $derived(
    mode === 'add' ? 'Add tool' : mode === 'edit' ? `Edit ${source?.assetTag}` : `Duplicate ${source?.assetTag}`,
  );
</script>

<Modal {title} size="lg" {onclose}>
  <form id="toolform" class="stack" onsubmit={save}>
    {#if mode === 'duplicate'}
      <p class="muted">A new tool with its own asset tag, copied from {source?.name}. Check its condition and serial.</p>
    {/if}
    <div class="grid">
      <div class="field span2">
        <label for="tf-name">Name</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input id="tf-name" class="input" bind:value={form.name} required autofocus placeholder="e.g. Pentalobe P5 driver" />
      </div>
      <div class="field">
        <label for="tf-cat">Category</label>
        <input id="tf-cat" class="input" bind:value={form.category} list="tf-cats" placeholder="e.g. Drivers" />
        <datalist id="tf-cats">
          {#each categories as c (c)}<option value={c}></option>{/each}
        </datalist>
      </div>
      <div class="field">
        <label for="tf-loc">Home location</label>
        <input id="tf-loc" class="input" bind:value={form.homeLocation} placeholder="e.g. Bench 2, top drawer" />
      </div>
      <div class="field">
        <label for="tf-loan">Default loan period</label>
        <select id="tf-loan" class="input" bind:value={form.defaultLoanMinutes}>
          {#each presets as p (p.minutes)}<option value={p.minutes}>{p.label}</option>{/each}
        </select>
      </div>
      <div class="field">
        <label for="tf-cost">Replacement cost</label>
        <input id="tf-cost" class="input" bind:value={cost} inputmode="decimal" placeholder="optional, e.g. 24.99" />
        {#if cost && !Number.isNaN(parseMoney(cost))}<span class="faint small">{money(parseMoney(cost), app.settings?.currencySymbol)}</span>{/if}
      </div>
      <div class="field">
        <label for="tf-model">Model</label>
        <input id="tf-model" class="input" bind:value={form.model} placeholder="optional" />
      </div>
      <div class="field">
        <label for="tf-serial">Serial number</label>
        <input id="tf-serial" class="input" bind:value={form.serial} placeholder="optional" />
      </div>
      <div class="field span2">
        <label for="tf-desc">Description</label>
        <textarea id="tf-desc" class="input" bind:value={form.description} rows="2" placeholder="optional"></textarea>
      </div>
    </div>
    <label class="check">
      <input type="checkbox" bind:checked={form.techOnly} />
      Tech-only — not loanable to borrowers
    </label>
    <PhotoInput bind:value={form.photoId} label="Tool photo (optional)" />
    {#if mode !== 'edit'}
      <ConditionPicker bind:value={form.condition} label="Current condition" />
      <div class="field">
        <label for="tf-cn">Condition notes</label>
        <input id="tf-cn" class="input" bind:value={form.conditionNotes} placeholder="optional, e.g. small scratch on handle" />
      </div>
    {/if}
    {#if error}<p class="error-text" role="alert">{error}</p>{/if}
  </form>
  {#snippet footer()}
    <button type="button" class="btn" onclick={onclose}>Cancel</button>
    <button form="toolform" class="btn btn-primary" disabled={saving}>{mode === 'edit' ? 'Save' : 'Add tool'}</button>
  {/snippet}
</Modal>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.9rem 1rem;
  }
  .span2 {
    grid-column: 1 / -1;
  }
  .small {
    font-size: 0.85rem;
  }
</style>
