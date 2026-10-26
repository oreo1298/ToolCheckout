<script lang="ts">
  import { api, type Settings, type ThemeSetting } from '../api';
  import { app } from '../app.svelte';
  import Icon from '../components/Icon.svelte';
  import { theme } from '../themeBoot';

  const initial = structuredClone($state.snapshot(app.settings!)) as Settings;
  const initialUnit: 'minutes' | 'hours' | 'days' =
    initial.defaultLoanMinutes % 1440 === 0 ? 'days' : initial.defaultLoanMinutes % 60 === 0 ? 'hours' : 'minutes';
  const UNIT = { minutes: 1, hours: 60, days: 1440 } as const;

  let draft = $state<Settings>(initial);
  let saving = $state(false);
  // Loan period edited as number + unit.
  let loanUnit = $state(initialUnit);
  let loanValue = $state(initial.defaultLoanMinutes / UNIT[initialUnit]);

  const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(app.settings) || loanMinutes() !== app.settings?.defaultLoanMinutes);

  function loanMinutes() {
    return Math.round(Number(loanValue) * UNIT[loanUnit]);
  }

  const LOCKS = [
    [30, '30 seconds'],
    [60, '1 minute'],
    [120, '2 minutes'],
    [300, '5 minutes'],
    [600, '10 minutes'],
    [900, '15 minutes'],
    [1800, '30 minutes'],
  ] as const;
  const REMINDERS = [
    [15, 'every 15 minutes'],
    [30, 'every 30 minutes'],
    [60, 'every hour'],
    [120, 'every 2 hours'],
    [240, 'every 4 hours'],
  ] as const;

  async function save(patch: Partial<Settings> = {}) {
    saving = true;
    try {
      const next = { ...$state.snapshot(draft), ...patch, defaultLoanMinutes: loanMinutes() } as Settings;
      const saved = await app.withTech(() => api.updateSettings(next));
      if (saved) {
        draft = structuredClone(saved);
        await app.refreshStatus();
        app.toast('Settings saved');
      }
    } catch (e) {
      app.error(e);
    } finally {
      saving = false;
    }
  }

  async function setTheme(t: ThemeSetting) {
    theme.setOverride(t); // preview immediately
    draft.theme = t;
    await save({ theme: t });
    theme.setOverride(app.settings!.theme);
  }
</script>

<section class="card card-pad stack">
  <h2>General</h2>

  <div class="field">
    <span class="label" id="theme-l">Appearance</span>
    <div class="segmented" role="radiogroup" aria-labelledby="theme-l">
      {#each [['system', 'System', 'monitor'], ['light', 'Light', 'sun'], ['dark', 'Dark', 'moon']] as [value, label, icon] (value)}
        <button
          role="radio"
          aria-checked={draft.theme === value}
          class:on={draft.theme === value}
          onclick={() => setTheme(value as ThemeSetting)}
        >
          <Icon name={icon} size={18} />
          {label}
        </button>
      {/each}
    </div>
    <span class="faint small">System follows your OS light/dark setting live.</span>
  </div>

  <div class="grid">
    <div class="field">
      <label for="shop">Shop name</label>
      <input id="shop" class="input" bind:value={draft.shopName} />
    </div>
    <div class="field">
      <label for="loan">Default loan period (new tools)</label>
      <div class="row">
        <input id="loan" class="input" type="number" min="1" bind:value={loanValue} style="max-width:110px" />
        <select class="input" bind:value={loanUnit} aria-label="Unit">
          <option value="minutes">minutes</option>
          <option value="hours">hours</option>
          <option value="days">days</option>
        </select>
      </div>
    </div>
    <div class="field">
      <label for="lock">Tech mode locks after</label>
      <select id="lock" class="input" bind:value={draft.autoLockSeconds}>
        {#each LOCKS as [v, l] (v)}<option value={v}>{l} of inactivity</option>{/each}
      </select>
    </div>
    <div class="field">
      <label for="cur">Currency symbol</label>
      <input id="cur" class="input" bind:value={draft.currencySymbol} style="max-width:110px" />
    </div>
  </div>

  <h3>Checkout</h3>
  <label class="check">
    <input type="checkbox" bind:checked={draft.borrowerPinRequired} />
    Borrowers confirm checkouts and drop-offs with their 4-digit PIN
  </label>
  <label class="check">
    <input type="checkbox" bind:checked={draft.techPinForCheckout} />
    Also require a technician PIN for every checkout
  </label>

  <h3>Reminders</h3>
  <div class="row wrap">
    <label class="check">
      <input type="checkbox" bind:checked={draft.overdueNotifications} />
      Desktop notification for overdue tools,
    </label>
    <select class="input" style="width:auto" bind:value={draft.overdueReminderMinutes} disabled={!draft.overdueNotifications} aria-label="Reminder interval">
      {#each REMINDERS as [v, l] (v)}<option value={v}>{l}</option>{/each}
    </select>
  </div>
  <div class="row wrap">
    <label class="check">
      <input type="checkbox" bind:checked={draft.endOfDayReminder} />
      End-of-day reminder listing everything still out, at
    </label>
    <input class="input" type="time" style="width:auto" bind:value={draft.closingTime} disabled={!draft.endOfDayReminder} aria-label="Closing time" />
  </div>

  <div class="row">
    <span class="spacer"></span>
    <button class="btn btn-primary" disabled={!dirty || saving} onclick={() => save()}>Save changes</button>
  </div>
</section>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 1rem;
  }
  .small {
    font-size: 0.85rem;
  }
  .segmented {
    display: inline-flex;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
    width: fit-content;
  }
  .segmented button {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    min-height: 44px;
    padding: 0 1rem;
    border: 0;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .segmented button + button {
    border-left: 1px solid var(--border-strong);
  }
  .segmented button.on {
    background: var(--accent);
    color: var(--accent-contrast);
  }
</style>
