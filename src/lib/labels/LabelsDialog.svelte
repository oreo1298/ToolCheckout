<script lang="ts">
  // Choose tools and a label stock, preview, print (or Save as PDF).
  import QRCode from 'qrcode';
  import { api, type Settings, type ToolRow } from '../api';
  import { app } from '../app.svelte';
  import Modal from '../components/Modal.svelte';
  import { printWith } from '../print';
  import LabelSheet from './LabelSheet.svelte';
  import { pageCss, STOCKS, stockById, type LabelItem } from './stocks';

  let { preselect = [], onclose }: { preselect?: number[]; onclose: () => void } = $props();

  let tools = $state<ToolRow[]>([]);
  const initialSelection = () => new Set(preselect);
  let selected = $state(initialSelection());
  let search = $state('');
  let stockId = $state(app.settings?.labelStock ?? 'avery5160');
  let withQr = $state(false);
  let skip = $state(0);
  let copies = $state(1);
  let outline = $state(false);
  let items = $state<LabelItem[]>([]);

  // Per-PC position calibration. WebKitGTK (Linux) ignores CSS @page margins
  // and adds GTK's default 0.25" (6.35 mm) margins, so start from -6.35 there.
  const CAL_KEY = 'toolcheckout.labelOffset';
  const platformDefault = app.boot?.platform === 'linux' ? -6.35 : 0;
  function loadCal(): { x: number; y: number } {
    try {
      const v = JSON.parse(localStorage.getItem(CAL_KEY) ?? 'null');
      if (v && typeof v.x === 'number' && typeof v.y === 'number') return v;
    } catch {
      /* storage unavailable: use defaults */
    }
    return { x: platformDefault, y: platformDefault };
  }
  const cal = loadCal();
  let offsetX = $state(cal.x);
  let offsetY = $state(cal.y);
  let showCal = $state(false);
  $effect(() => {
    try {
      localStorage.setItem(CAL_KEY, JSON.stringify({ x: Number(offsetX) || 0, y: Number(offsetY) || 0 }));
    } catch {
      /* not critical */
    }
  });

  const stock = $derived(stockById(stockId));
  const shown = $derived(
    search.trim()
      ? tools.filter((t) => `${t.assetTag} ${t.name} ${t.category}`.toLowerCase().includes(search.trim().toLowerCase()))
      : tools,
  );

  $effect(() => {
    api.listTools({ limit: 5000 }).then((t) => (tools = t), app.error);
  });

  // Rebuild label contents (QR codes are rendered asynchronously).
  $effect(() => {
    const chosen = tools.filter((t) => selected.has(t.id));
    const n = Math.max(1, Math.min(10, copies));
    const qr = withQr && stock.roomy;
    let cancelled = false;
    Promise.all(
      chosen.map(async (t) => ({
        tag: t.assetTag,
        name: t.name,
        qrSvg: qr ? await QRCode.toString(t.assetTag, { type: 'svg', margin: 0, errorCorrectionLevel: 'M' }) : null,
      })),
    ).then((list) => {
      if (!cancelled) items = list.flatMap((l) => Array.from({ length: n }, () => l));
    });
    return () => (cancelled = true);
  });

  function toggle(id: number) {
    const s = new Set(selected);
    if (s.has(id)) s.delete(id);
    else s.add(id);
    selected = s;
  }

  function selectShown() {
    selected = new Set([...selected, ...shown.map((t) => t.id)]);
  }

  async function print() {
    await printWith(
      LabelSheet,
      { stock, items, skip, shopName: app.settings?.shopName ?? '', outline, offsetX: Number(offsetX) || 0, offsetY: Number(offsetY) || 0 },
      pageCss(stock),
    );
    // Remember the stock for next time.
    if (app.session && app.settings && app.settings.labelStock !== stockId) {
      const s: Settings = { ...app.settings, labelStock: stockId };
      api.updateSettings(s).then(() => app.refreshStatus(), () => {});
    }
  }
</script>

<Modal title="Print asset tag labels" size="xl" {onclose}>
  <div class="cols">
    <section class="stack">
      <div class="row">
        <input class="input" type="search" placeholder="Filter tools…" bind:value={search} aria-label="Filter tools" />
        <button class="btn btn-sm" onclick={selectShown}>Select all</button>
        <button class="btn btn-sm btn-ghost" onclick={() => (selected = new Set())}>None</button>
      </div>
      <ul class="pick">
        {#each shown as t (t.id)}
          <li>
            <label class="check">
              <input type="checkbox" checked={selected.has(t.id)} onchange={() => toggle(t.id)} />
              <span class="mono">{t.assetTag}</span>
              <span class="grow">{t.name}</span>
            </label>
          </li>
        {/each}
      </ul>
      <p class="faint">{selected.size} selected · {items.length} labels</p>
    </section>
    <section class="stack">
      <div class="field">
        <label for="lb-stock">Label stock</label>
        <select id="lb-stock" class="input" bind:value={stockId}>
          {#each STOCKS as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
        </select>
      </div>
      <div class="row wrap">
        <label class="check"><input type="checkbox" bind:checked={withQr} disabled={!stock.roomy} /> Add QR code{stock.roomy ? '' : ' (too small on this stock)'}</label>
      </div>
      <div class="row wrap">
        <div class="field">
          <label for="lb-copies">Copies of each</label>
          <input id="lb-copies" class="input" type="number" min="1" max="10" bind:value={copies} style="width:100px" />
        </div>
        <div class="field">
          <label for="lb-skip">Skip used labels</label>
          <input id="lb-skip" class="input" type="number" min="0" max={stock.cols * stock.rows - 1} bind:value={skip} style="width:100px" />
        </div>
      </div>
      <label class="check"><input type="checkbox" bind:checked={outline} /> Draw label outlines (for a test print on plain paper)</label>
      <p class="muted small">
        In the print dialog choose the right paper size ({stock.page === 'a4' ? 'A4' : 'Letter'}) and
        <strong>100% / Actual size</strong> (not "Fit to page"), otherwise the labels won't line up.
        <button class="linkish" onclick={() => (showCal = !showCal)}>Labels slightly off? Adjust position</button>
      </p>
      {#if showCal}
        <div class="row wrap cal">
          <div class="field">
            <label for="lb-ox">Move right (mm)</label>
            <input id="lb-ox" class="input" type="number" step="0.5" bind:value={offsetX} style="width:110px" />
          </div>
          <div class="field">
            <label for="lb-oy">Move down (mm)</label>
            <input id="lb-oy" class="input" type="number" step="0.5" bind:value={offsetY} style="width:110px" />
          </div>
          <button class="btn btn-sm btn-ghost" onclick={() => ((offsetX = platformDefault), (offsetY = platformDefault))}>Reset</button>
          <span class="faint small">Print a test page with outlines on plain paper, hold it over a label sheet, and adjust. Saved on this PC.</span>
        </div>
      {/if}
      <div class="preview" aria-label="Preview of the first page">
        <div class="scaler" style="width:{stock.pageW}mm;height:{stock.pageH}mm">
          <LabelSheet {stock} items={items.slice(0, stock.cols * stock.rows - Math.min(skip, stock.cols * stock.rows - 1))} {skip} shopName={app.settings?.shopName ?? ''} outline />
        </div>
      </div>
    </section>
  </div>
  {#snippet footer()}
    <button class="btn" onclick={onclose}>Close</button>
    <button class="btn btn-primary" disabled={!items.length} onclick={print}>Print {items.length} labels</button>
  {/snippet}
</Modal>

<style>
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.2fr);
    gap: 1.25rem;
  }
  .pick {
    list-style: none;
    margin: 0;
    padding: 0.25rem;
    max-height: 460px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .pick .check {
    min-height: 40px;
    width: 100%;
    padding: 0 0.4rem;
  }
  .small {
    font-size: 0.85rem;
  }
  .linkish {
    border: 0;
    background: none;
    padding: 0;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
    text-decoration: underline;
  }
  .cal {
    align-items: flex-end;
  }
  .preview {
    position: relative;
    height: 420px;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-3);
  }
  /* Absolutely positioned so the full-size page doesn't widen the layout. */
  .scaler {
    position: absolute;
    left: 50%;
    top: 10px;
    transform: translateX(-50%) scale(0.42);
    transform-origin: top center;
    box-shadow: 0 2px 10px rgb(0 0 0 / 0.25);
  }
</style>
